#!/usr/bin/env python3
"""Run the pinned host overlay merger with production slime contributions.

World-pack discovery is a stub; the host merge function is copied unchanged
from a verified clean pinned checkout. This is not a complete Game reload.
"""
import json
import os
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]
HOST = Path(os.environ['HORIZON_HOST']).resolve()
PIN = json.loads((ROOT / 'compatibility.json').read_text())['horizon_revision']
assert subprocess.check_output(['git', '-C', str(HOST), 'rev-parse', 'HEAD'], text=True).strip() == PIN
assert not subprocess.check_output(['git', '-C', str(HOST), 'status', '--porcelain', '--untracked-files=no'], text=True).strip()
source = (HOST / 'crates/horizon-client/src/game/functions.rs').read_text()
start = source.index('pub(super) fn overlay_files(')
end = source.index('\nimpl FnState {', start)
merger = source[start:end]
assert merger.rstrip().endswith('}')
with tempfile.TemporaryDirectory(prefix='slime-overlay-') as directory:
    work = Path(directory)
    manifest = f'''[package]
name = "slime-overlay-fixture"
version = "0.1.0"
edition = "2021"
[dependencies]
serde_json = "1"
log = "0.4"
tconstruct-source-config = {{ path = {json.dumps(str(ROOT / 'mixins/tconstruct/source-config-parser'))} }}
[lib]
path = "lib.rs"
'''
    (work / 'Cargo.toml').write_text(manifest)
    # The main fixture fetches this identical dependency set in CI. Keep the
    # temporary host fixture on its checked-in versions instead of resolving anew.
    lock = (ROOT / 'tests/slime-crafting/Cargo.lock').read_text()
    lock = lock.replace('name = "tconstruct-slime-crafting-tests"', 'name = "slime-overlay-fixture"')
    (work / 'Cargo.lock').write_text(lock)
    rust = '#![cfg(test)]\n#[path = ' + json.dumps(str(ROOT / 'mixins/tconstruct/source-slime-crafting.rs')) + ']\nmod slime;\n'
    rust += '''mod host {
fn content_overlay(world_root: &std::path::Path, _: &std::path::Path, _: bool, _: &[String]) -> Vec<(String, Vec<Vec<u8>>)> {
    std::fs::read(world_root.join("pack-files.json")).ok()
        .map(|bytes| serde_json::from_slice(&bytes).unwrap()).unwrap_or_default()
}
'''+merger+'\n}\n'
    rust += r'''
#[test]
fn host_overlay_keeps_pack_priority_and_rebuilds_without_disabled_recipe() {
    let root = std::env::temp_dir().join(format!("slime-overlay-world-{}", std::process::id()));
    std::fs::create_dir_all(root.join("config")).unwrap();
    let id = "recipe/tconstruct:common/slime/lead";
    let replacement = br#"{"type":"minecraft:crafting_shapeless","ingredients":["minecraft:diamond"],"result":{"id":"minecraft:lead","count":1}}"#.to_vec();
    let pack = vec![(id.to_owned(), vec![replacement.clone()])];
    std::fs::write(root.join("pack-files.json"), serde_json::to_vec(&pack).unwrap()).unwrap();
    for enabled in [true, false, true, false] {
        std::fs::write(root.join("config/tconstruct-common.toml"), format!("[recipes]\nglassRecipeFix = {enabled}")).unwrap();
        let mut files = Vec::new();
        slime::contribute(&root, &mut files);
        assert_eq!(files.len(), if enabled { 2 } else { 0 });
        let overlay = host::overlay_files(&root, &root, false, &[], files);
        let lead = &overlay.iter().find(|(path, _)| path == id).unwrap().1;
        assert_eq!(lead.last(), Some(&replacement));
        assert_eq!(lead.len(), if enabled { 2 } else { 1 });
        assert_eq!(overlay.iter().any(|(path, _)| path.ends_with("/sticky_piston")), enabled);
    }
    std::fs::remove_dir_all(root).unwrap();
}
'''
    (work / 'lib.rs').write_text(rust)
    subprocess.run(['cargo', 'test', '--locked', '--offline', '--manifest-path', str(work / 'Cargo.toml'), '--', '--nocapture'], check=True)
