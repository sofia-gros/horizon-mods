#!/usr/bin/env python3
"""Prepare an UNAPPLIED prototype excerpt. Never writes to an official checkout."""
import argparse
import difflib
import hashlib
import json
from pathlib import Path
import subprocess

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[2]
REVISION = '624c4f6265c02296fbe8a80f029dd0d987fb94c0'
SOURCE = 'crates/horizon-client-render/src/effects.rs'
SUPPORT = 'crates/horizon-client-render/src/effects/extension.rs'
SHA = '5269c3c35f38c4c658d4409b0f7badae4f0ddca0b68df36c5197a5dd51f735dd'
STOP = '/// What an effect tick asks the game to do to the player.'
IMPORT = '''#[path = "effects/extension.rs"]
mod extension;
pub use extension::{Extensions as EffectExtensions, Limits as EffectExtensionLimits,
    Error as EffectExtensionError, Record as EffectRecord, Layer as EffectRecordLayer};
'''

def digest(value): return hashlib.sha256(value).hexdigest()

def replace_once(text, old, new):
    if text.count(old) != 1: raise ValueError('expected exactly one selected native span')
    return text.replace(old, new, 1)

def source(official):
    official = official.resolve()
    if subprocess.check_output(['git', '-C', str(official), 'rev-parse', 'HEAD'], text=True).strip() != REVISION:
        raise ValueError('wrong approved official revision')
    if subprocess.check_output(['git', '-C', str(official), 'status', '--porcelain', '--untracked-files=no'], text=True):
        raise ValueError('official tracked sources must be clean')
    inventory = json.loads((REPO / 'pluto-lib/docs/effect-extension-callsites.json').read_text())
    for name, expected in inventory['source_files'].items():
        if digest((official / name).read_bytes()) != expected:
            raise ValueError('inventoried official source changed: ' + name)
    raw = (official / SOURCE).read_bytes()
    if digest(raw) != SHA: raise ValueError('native effect source changed')
    return raw.decode()

def proposed(original):
    text = replace_once(original, 'use horizon_client_core::potions;\n', 'use horizon_client_core::potions;\n' + IMPORT)
    text = replace_once(text, '#[derive(Clone, Debug, PartialEq)]\npub struct EffectInstance {', '#[derive(Clone, Debug)]\npub struct EffectInstance {')
    text = replace_once(text, '    pub hidden: Option<Box<EffectInstance>>,\n', '    pub hidden: Option<Box<EffectInstance>>,\n    /// Proposed owned opaque data. Not yet integrated with native saves or wire.\n    pub extensions: EffectExtensions,\n')
    text = replace_once(text, 'impl EffectInstance {\n', '''// Preserve the existing native equality contract. Payload equality is explicit.
impl PartialEq for EffectInstance {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id && self.duration == other.duration
            && self.amplifier == other.amplifier && self.ambient == other.ambient
            && self.visible == other.visible && self.hidden == other.hidden
    }
}
impl EffectInstance {
''')
    text = replace_once(text, '            visible: true,\n            hidden: None,\n', '            visible: true,\n            hidden: None,\n            extensions: EffectExtensions::default(),\n')
    text = replace_once(text, '            self.visible = h.visible;\n            self.hidden = h.hidden;', '            self.visible = h.visible;\n            self.extensions = h.extensions;\n            self.hidden = h.hidden;')
    return text

def patch(original, changed):
    support = (HERE / 'extension.rs').read_text()
    return ''.join(difflib.unified_diff(original.splitlines(True), changed.splitlines(True),
        fromfile='a/' + SOURCE, tofile='b/' + SOURCE)) + ''.join(difflib.unified_diff(
        [], support.splitlines(True), fromfile='/dev/null', tofile='b/' + SUPPORT))

def fixture(changed):
    excerpt = changed[:changed.index(STOP)]
    # The selected native instance implementation does not use the potion catalog.
    # Remove only that unused import, not any native function or native type.
    excerpt = replace_once(excerpt, 'use horizon_client_core::potions;\n', '')
    excerpt = replace_once(excerpt, '#[path = "effects/extension.rs"]', '#[path = "extension.rs"]')
    return excerpt + '\n#[cfg(test)]\n#[path = "tests.rs"]\nmod proposal_tests;\n'

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--official', type=Path, required=True)
    parser.add_argument('--write-patch', action='store_true')
    parser.add_argument('--output', type=Path)
    args = parser.parse_args()
    original = source(args.official)
    changed = proposed(original)
    delta = patch(original, changed)
    target = HERE / 'UNAPPLIED-player-instance.patch'
    if args.write_patch:
        target.write_text(delta)
    elif target.read_text() != delta:
        raise ValueError('prototype patch does not exactly match its pinned inputs')
    if args.output:
        output = args.output.resolve()
        if not output.is_relative_to((REPO / '.work').resolve()):
            raise ValueError('generated excerpt must stay in this MOD worktree .work directory')
        if output.exists(): raise ValueError('output must not exist')
        output.mkdir(parents=True)
        generated = fixture(changed).encode()
        (output / 'fixture.rs').write_bytes(generated)
        for name in ('extension.rs', 'tests.rs'):
            (output / name).write_bytes((HERE / name).read_bytes())
        evidence = {
            'status': 'UNAPPLIED/UNFINISHED prototype fixture, not native gameplay',
            'official_revision': REVISION, 'official_source': SOURCE,
            'official_sha256': digest(original.encode()),
            'original_instance_excerpt_sha256': digest(original[:original.index(STOP)].encode()),
            'proposed_full_file_sha256': digest(changed.encode()),
            'patch_sha256': digest(delta.encode()),
            'prepared_files': {p.name: digest(p.read_bytes()) for p in sorted(output.iterdir())},
            'scope': 'Exact pinned native EffectInstance excerpt plus proposed changes and real std types. No substituted native type. Unused potion import removed; no potion resolver mock.',
            'excluded': ['Full native crate type checking', 'PlayerEffects/Level/NBT save migration', 'Native contact or cure activation', 'LAN/remote protocol authority', 'Mob representation migration'],
        }
        (output / 'evidence.json').write_text(json.dumps(evidence, indent=2) + '\n')
    # Read-only source identity check again after all MOD-local writes.
    if source(args.official) != original: raise ValueError('official input changed during preparation')
    print('Unapplied patch and selected native excerpt verified; no official writes or compilation.')

if __name__ == '__main__':
    main()
