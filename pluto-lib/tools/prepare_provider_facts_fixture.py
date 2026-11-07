#!/usr/bin/env python3
"""Prepare exact packaged native provider facts without client-lib test codegen.

Native ItemView is supplied by the whole unchanged official advancements.rs,
compiled as a dependency with actual core potion/NBT reexports. All converter,
equipment and contribution modules remain unchanged; no native facts are stubbed.
This is a bounded module test, not native hook wiring or game/runtime evidence.
"""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import tomllib

ROOT = Path(__file__).resolve().parents[1]
ORIGIN = 'https://github.com/horizon-rust/horizon.git'
HOST_ADVANCEMENTS = 'crates/horizon-client/src/advancements.rs'
MODULES = {
    'stack-storage.rs': ('pluto_native_creative-stack-storage', 'pluto_stack_storage'),
    'component-view.rs': ('pluto_native_creative-component-view', 'pluto_component_view'),
    'equipment-snapshot.rs': ('pluto_native_creative-equipment-snapshot', 'pluto_equipment'),
    'protection-provider-facts.rs': ('pluto_native_creative-protection-provider-facts', 'pluto_protection_facts'),
    'protection-contribution.rs': ('pluto_native_creative-protection-contribution', 'pluto_protection_contribution'),
}
LOCAL = {('pluto-provider-facts-tests', '0.0.0', None, None),
         ('pluto-provider-native-fixture', '0.0.0', None, None),
         ('pluto-inventory-api', '0.1.0', None, None)}


def digest(data):
    return hashlib.sha256(data).hexdigest()


def identity(package):
    return tuple(package.get(k) for k in ('name', 'version', 'source', 'checksum'))


def clean_official(host, revision):
    def git(*args):
        return subprocess.check_output(['git', '-C', str(host), *args], text=True).strip()
    if (git('remote', 'get-url', 'origin') != ORIGIN or git('rev-parse', 'HEAD') != revision
            or git('status', '--porcelain', '--untracked-files=all')):
        raise ValueError('Require the clean selected official Horizon checkout')


def files(root):
    paths = sorted(root.rglob('*'))
    if any(p.is_symlink() for p in paths):
        raise ValueError('Exact fixture/stage sources may not contain symlinks')
    return {str(p.relative_to(root)): digest(p.read_bytes()) for p in paths if p.is_file()}


def check_hashes(root, expected):
    for name, sha in expected.items():
        path = root / name
        if path.is_symlink() or not path.is_file() or digest(path.read_bytes()) != sha:
            raise ValueError(f'Exact source changed: {path}')


def metadata(output, locked):
    args = ['cargo', 'metadata', '--offline', '--manifest-path', str(output / 'Cargo.toml'), '--format-version', '1']
    if locked:
        args.append('--locked')
    return json.loads(subprocess.check_output(args, text=True))


def check_dependencies(output, host, stage, api, locked):
    resolved = metadata(output, locked)
    local_paths = {
        'pluto-provider-facts-tests': output / 'Cargo.toml',
        'pluto-provider-native-fixture': output / 'native/Cargo.toml',
        'pluto-inventory-api': api / 'Cargo.toml',
    }
    found = set()
    for package in resolved['packages']:
        path = Path(package['manifest_path']).resolve()
        if package['name'] in local_paths:
            if package['source'] is not None or path != local_paths[package['name']].resolve():
                raise ValueError('Local package did not resolve to its exact staged/fixture source')
            found.add(package['name'])
        elif package['source'] is None and not path.is_relative_to(stage):
            raise ValueError('Native path dependency is outside the exact staged official package')
    if found != set(local_paths):
        raise ValueError('Required exact local package missing')
    official = {identity(p) for p in tomllib.loads((host / 'Cargo.lock').read_text())['package']}
    actual = {identity(p) for p in tomllib.loads((output / 'Cargo.lock').read_text())['package']}
    if actual - official - LOCAL or not LOCAL.issubset(actual):
        raise ValueError('Fixture identities differ from official lock and exact local packages')


def verify(output):
    evidence = json.loads((output / 'fixture-sources.json').read_text())
    host, stage, api = [Path(evidence[k]) for k in ('official_source', 'staged_source', 'inventory_api')]
    clean_official(host, evidence['official_revision'])
    if digest((host / 'Cargo.lock').read_bytes()) != evidence['official_lock_sha256']:
        raise ValueError('Official lock changed')
    original = (host / HOST_ADVANCEMENTS).read_bytes()
    if digest(original) != evidence['official_advancements_sha256'] or (output / 'native/advancements.rs').read_bytes() != original:
        raise ValueError('Complete official advancements source is not byte-identical')
    check_hashes(stage, evidence['stage_files_sha256'])
    check_hashes(output, evidence['fixture_files_sha256'])
    check_dependencies(output, host, stage, api, locked=True)
    return evidence


def prepare(host, stage, cutoff_path, output):
    host, stage, cutoff_path, output = [p.resolve() for p in (host, stage, cutoff_path, output)]
    cutoff = json.loads(cutoff_path.read_text())
    clean_official(host, cutoff['horizon'])
    archive = cutoff_path.with_name('plutolib.hnmod')
    if digest(archive.read_bytes()) != cutoff['archive_sha256']:
        raise ValueError('Packaged archive differs from the verified cutoff')
    stage_metadata = json.loads((stage / '.horizon-mixins-stage.json').read_text())
    if (stage / '.horizon-revision').read_text().strip() != cutoff['horizon']:
        raise ValueError('Stage revision differs from the exact official cutoff')
    # Stage inputs prove the transformer started from this official checkout.
    for name, sha in stage_metadata['inputs'].items():
        path = host / name
        if name == '.horizon-revision':
            data = (cutoff['horizon'] + '\n').encode()
        elif path.is_file():
            data = path.read_bytes()
        else:
            raise ValueError(f'Stage input missing from official source: {name}')
        if digest(data) != sha:
            raise ValueError(f'Stage input differs from official source: {name}')
    if output.exists():
        raise ValueError('Output must be new')
    native = output / 'native'
    native.mkdir(parents=True)
    modules = {m['module_id']: m for m in stage_metadata['modules']}
    declarations = []
    for filename, (module_id, name) in MODULES.items():
        source = (ROOT / 'mixins/pluto_native_creative' / filename).read_bytes()
        if modules[module_id]['source'].encode() != source:
            raise ValueError(f'Pluto module differs from the exact packaged source: {filename}')
        (output / filename).write_bytes(source)
        declarations.append(f'#[path={json.dumps(filename)}] pub mod {name};')
    api_entries = [c for c in stage_metadata['crates'] if c['package'] == 'pluto-inventory-api']
    if len(api_entries) != 1:
        raise ValueError('Require exactly one actual packaged inventory crate')
    api = stage / api_entries[0]['path']
    check_hashes(api, api_entries[0]['files'])
    check_hashes(ROOT / 'mixins/pluto_native_creative/inventory-api', api_entries[0]['files'])
    original = (host / HOST_ADVANCEMENTS).read_bytes()
    (native / 'advancements.rs').write_bytes(original)
    (native / 'lib.rs').write_text('pub use horizon_client_core::{potions as brewing, nbt_path};\npub mod advancements;\n')
    (native / 'Cargo.toml').write_text('[package]\nname="pluto-provider-native-fixture"\nversion="0.0.0"\nedition="2024"\n[lib]\npath="lib.rs"\n[dependencies]\nserde_json="1"\nlog="0.4"\n'
        'horizon-client-core={path=' + json.dumps(str(stage / 'crates/horizon-client-core')) + '}\n'
        'horizon-world={path=' + json.dumps(str(stage / 'crates/horizon-world')) + '}\n')
    (output / 'provider-facts-tests.rs').write_bytes((ROOT / 'tests/native/provider_facts.rs').read_bytes())
    declarations.append('#[path="provider-facts-tests.rs"] mod cross_service_tests;')
    (output / 'lib.rs').write_text('pub use horizon_items::{items, item_data};\npub use pluto_provider_native_fixture::advancements;\npub use pluto_inventory_api as pluto_inventory;\n' + '\n'.join(declarations) + '\n')
    (output / 'Cargo.toml').write_text('[package]\nname="pluto-provider-facts-tests"\nversion="0.0.0"\nedition="2024"\n[workspace]\n[lib]\npath="lib.rs"\n[dependencies]\nserde_json="1"\n'
        'pluto-provider-native-fixture={path="native"}\n'
        'pluto-inventory-api={path=' + json.dumps(str(api)) + '}\n'
        'horizon-items={path=' + json.dumps(str(stage / 'crates/horizon-items')) + '}\n'
        'horizon-world={path=' + json.dumps(str(stage / 'crates/horizon-world')) + '}\n')
    patches = tomllib.loads((stage / 'Cargo.toml').read_text()).get('patch', {}).get('crates-io', {})
    with (output / 'Cargo.toml').open('a') as manifest:
        manifest.write('\n[patch.crates-io]\n')
        for name, entry in patches.items():
            if set(entry) != {'path'}:
                raise ValueError('Only exact staged native path patches are supported')
            path = (stage / entry['path']).resolve()
            if not path.is_relative_to(stage):
                raise ValueError('Native path patch escapes the approved stage')
            manifest.write(json.dumps(name) + '={path=' + json.dumps(str(path)) + '}\n')
    evidence = {
        'cutoff': cutoff, 'official_source': str(host), 'official_revision': cutoff['horizon'],
        'staged_source': str(stage), 'inventory_api': str(api),
        'official_advancements_sha256': digest(original),
        'official_lock_sha256': digest((host / 'Cargo.lock').read_bytes()),
        'stage_files_sha256': files(stage), 'fixture_files_sha256': files(output),
        'scope': 'Exact native component/equipment/provider/contribution modules, full official ItemView source, real staged world damage catalog and Policy. No client hook dispatch or game/runtime proof.',
    }
    (output / 'Cargo.lock').write_bytes((host / 'Cargo.lock').read_bytes())
    check_dependencies(output, host, stage, api, locked=False)
    (output / 'fixture-sources.json').write_text(json.dumps(evidence, indent=2) + '\n')
    verify(output)
    print(output)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--horizon-source', type=Path)
    parser.add_argument('--staged-source', type=Path)
    parser.add_argument('--cutoff', type=Path)
    parser.add_argument('--output', required=True, type=Path)
    parser.add_argument('--verify', action='store_true')
    args = parser.parse_args()
    if args.verify:
        verify(args.output.resolve())
        print('Exact native module/source/API/lock identities verified')
    elif any(p is None for p in (args.horizon_source, args.staged_source, args.cutoff)):
        parser.error('--horizon-source, --staged-source and --cutoff are required')
    else:
        prepare(args.horizon_source, args.staged_source, args.cutoff, args.output)
