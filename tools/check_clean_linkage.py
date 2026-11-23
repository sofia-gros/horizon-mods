"""Typecheck the checked clean consumer against a prepared PlutoLib API snapshot.

This extends an isolated diagnostic workspace once. It is not the official
launcher's generated executable, a linked game binary, or gameplay evidence.
"""
import argparse
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import tomllib

from port import checked_files

ROOT = Path(__file__).resolve().parents[1]

# The pipes consumer bundle, laid out exactly like the Tinkers bundle
# (port.BUNDLE): <mod>/mixins/<mod>/{horizon-mixin.toml, api/}.
PIPES_BUNDLE = Path('pipes/mixins/pipes')


def prepare(prepared):
    prepared = prepared.resolve()
    if not prepared.is_relative_to(ROOT / '.local/clean-port'):
        raise ValueError('consumer checks require an isolated clean-port workspace')
    result = json.loads((prepared / 'api-check.json').read_text(encoding='utf-8'))
    if result['exit'] != 0 or result['package'] != 'mod-pluto_lib':
        raise ValueError('successful PlutoLib API typecheck required')
    files, _, _, _ = checked_files(ROOT)
    stage = prepared / 'applied'
    workspace = stage / 'Cargo.toml'
    original = workspace.read_text(encoding='utf-8')
    if original.count('members = [\n') != 1:
        raise ValueError('workspace member declaration changed')
    consumer = stage / 'native-api/tconstruct'
    pipes_consumer = stage / 'native-api/pipes'
    probe = stage / 'native-api/clean-linkage'
    if consumer.exists() or pipes_consumer.exists() or probe.exists():
        raise ValueError('consumer check already staged; preserve its evidence')
    manifest = files['api/Cargo.toml'].decode('utf-8')
    parsed = tomllib.loads(manifest)
    expected = {'git': 'https://github.com/horizon-rust/horizon',
                'rev': '441f49d57e344bd56b90fd4af5c4e443ff3937c9', 'default-features': False}
    # Reviewed ordinary dependency: preserve the public JSON API's exact version.
    # Keep an exact allowlist here; do not silently project future dependencies.
    if parsed['dependencies'] != {'horizon-client': expected, 'serde_json': '=1.0.154', 'toml': '=0.8.23'}:
        raise ValueError('consumer dependencies changed; review launcher projection')
    lines = manifest.splitlines(keepends=True)
    indexes = [i for i, line in enumerate(lines) if line.startswith('horizon-client = ')]
    if len(indexes) != 1:
        raise ValueError('ambiguous consumer host dependency')
    # Pinned launcher native_mods/api.rs rewrites the host, then dependencies.rs
    # adds the declared bundle edge under its ID and actual Cargo package name.
    lines[indexes[0]] = ('horizon-client = { path = "../../crates/horizon-client", default-features = false }\n'
                         'pluto_lib = { package = "mod-pluto_lib", path = "../pluto_lib" }\n')
    for name, data in files.items():
        if name.startswith('api/'):
            dest = consumer / name.removeprefix('api/')
            dest.parent.mkdir(parents=True, exist_ok=True)
            dest.write_bytes(data)
    (consumer / 'Cargo.toml').write_text(''.join(lines), encoding='utf-8', newline='\n')
    # Second consumer (build-test composition): the pipes mod's API crate,
    # staged with the same host rewrite and bundle edge as the tconstruct
    # consumer. Its manifest carries the same reviewed dependency allowlist.
    pipes_source = ROOT / PIPES_BUNDLE / 'api'
    pipes_manifest = (pipes_source / 'Cargo.toml').read_text(encoding='utf-8')
    pipes_parsed = tomllib.loads(pipes_manifest)
    if pipes_parsed['package']['name'] != 'mod-pipes':
        raise ValueError('pipes API package name changed; review launcher projection')
    if pipes_parsed['dependencies'] != {'horizon-client': expected, 'serde_json': '=1.0.154', 'toml': '=0.8.23'}:
        raise ValueError('pipes consumer dependencies changed; review launcher projection')
    pipes_lines = pipes_manifest.splitlines(keepends=True)
    pipes_indexes = [i for i, line in enumerate(pipes_lines) if line.startswith('horizon-client = ')]
    if len(pipes_indexes) != 1:
        raise ValueError('ambiguous pipes consumer host dependency')
    pipes_lines[pipes_indexes[0]] = ('horizon-client = { path = "../../crates/horizon-client", default-features = false }\n'
                                     'pluto_lib = { package = "mod-pluto_lib", path = "../pluto_lib" }\n')
    shutil.copytree(pipes_source, pipes_consumer)
    (pipes_consumer / 'Cargo.toml').write_text(''.join(pipes_lines), encoding='utf-8', newline='\n')
    (probe / 'src').mkdir(parents=True)
    (probe / 'Cargo.toml').write_text(newline='\n', data='''[package]
name = "pluto-clean-linkage"
version = "0.0.0"
edition = "2024"
publish = false
[dependencies]
horizon-client = { path = "../../crates/horizon-client", default-features = false }
pluto_lib = { package = "mod-pluto_lib", path = "../pluto_lib" }
tconstruct = { package = "mod-tconstruct", path = "../tconstruct" }
pipes = { package = "mod-pipes", path = "../pipes" }
''', encoding='utf-8')
    (probe / 'src/main.rs').write_text(newline='\n', data='''// Diagnostic type composition only; never launch this as the game.
fn main() {
    let _mods: horizon_client::mod_api::Mods = vec![
        pluto_lib::hooks(), tconstruct::hooks(), pipes::hooks(),
    ].into();
}
''', encoding='utf-8')
    workspace.write_text(newline='\n', data=original.replace('members = [\n',
        'members = [\n    "native-api/tconstruct",\n    "native-api/pipes",\n    "native-api/clean-linkage",\n', 1),
        encoding='utf-8')
    return stage, {name: hashlib.sha256(data).hexdigest() for name, data in files.items()}


def check(prepared):
    stage, inputs = prepare(prepared)
    command = ['cargo', '+1.95.0', 'check', '--offline', '--manifest-path',
               str(stage / 'Cargo.toml'), '-p', 'pluto-clean-linkage']
    code = subprocess.run(command).returncode
    if code == 0:
        code = subprocess.run(command[:4] + ['--locked'] + command[4:]).returncode
    (prepared / 'consumer-check.json').write_text(newline='\n', data=json.dumps({
        'exit': code, 'consumer_inputs': inputs,
        'launcher_integration_checked': False, 'binary_linked': False,
        'gameplay_checked': False,
    }, indent=2) + '\n', encoding='utf-8')
    return code


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--prepared', required=True, type=Path)
    args = parser.parse_args()
    raise SystemExit(check(args.prepared.resolve()))
