#!/usr/bin/env python3
"""Prepare additive exact snapshot tests against a real applied native items crate."""
import argparse
import json
import re
from pathlib import Path
import tomllib
from prepare_provider_facts_fixture import clean_official, files, check_hashes, digest, identity, metadata
import check_wire_snapshot_plan as guard

ROOT = Path(__file__).resolve().parents[1]
REQUIRED_TESTS = (
    'native_reader_borrows_the_actual_baseline_and_patch',
    'different_stale_baselines_with_identical_public_wire_facts_reject',
    'independent_allocations_compare_by_values_not_addresses',
    'baseline_count_component_and_bundle_selection_changes_reject',
    'every_ordinary_nested_stack_field_reaches_private_baselines',
    'retained_baselines_can_themselves_contain_retained_baselines',
    'repeated_baseline_graphs_finish_without_expanding_every_path',
    'exact_read_comparison_does_not_relax_save_or_change_inputs',
)
LOCAL = {('pluto-stack-snapshot-tests', '0.0.0', None, None), ('pluto-inventory-api', '0.1.0', None, None)}


def require_tests(output):
    for name in REQUIRED_TESTS:
        if not re.search(r'^test boundary::tests::' + name + r' \.\.\. ok$', output, re.MULTILINE):
            raise ValueError(f'Required real native test did not pass: {name}')
    if not re.search(r'^test result: ok\. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;', output, re.MULTILINE):
        raise ValueError('Expected all eight real native tests with none filtered or ignored')


def dependencies(output, official, stage, api, locked):
    paths = {'pluto-stack-snapshot-tests': output / 'Cargo.toml', 'pluto-inventory-api': api / 'Cargo.toml'}
    found = set()
    for package in metadata(output, locked)['packages']:
        path = Path(package['manifest_path']).resolve()
        if package['name'] in paths:
            if package['source'] is not None or path != paths[package['name']].resolve():
                raise ValueError('Fixture/API path differs from the exact selected package')
            found.add(package['name'])
        elif package['source'] is None and not path.is_relative_to(stage):
            raise ValueError('Native path dependency is outside the approved applied stage')
    if found != set(paths):
        raise ValueError('Missing actual fixture/API package')
    allowed = {identity(p) for p in tomllib.loads((official / 'Cargo.lock').read_text())['package']}
    actual = {identity(p) for p in tomllib.loads((output / 'Cargo.lock').read_text())['package']}
    if actual - allowed - LOCAL or not LOCAL.issubset(actual):
        raise ValueError('Dependency identities differ from official lock plus actual local packages')


def verify(output):
    evidence = json.loads((output / 'fixture-sources.json').read_text())
    official, stage, api = [Path(evidence[k]) for k in ('official', 'stage', 'api')]
    clean_official(official, evidence['cutoff']['horizon'])
    if digest((official / 'Cargo.lock').read_bytes()) != evidence['official_lock_sha256']:
        raise ValueError('Official lock changed')
    if files(stage) != evidence['stage_files_sha256']:
        raise ValueError('Applied stage changed from the exact full-file seal')
    check_hashes(output, evidence['fixture_files_sha256'])
    dependencies(output, official, stage, api, locked=True)


def prepare(official, stage, cutoff, stage_seal, output):
    official, stage, cutoff, output = [p.resolve() for p in (official, stage, cutoff, output)]
    source = json.loads(cutoff.read_text())
    if Path(source['stage_path']).resolve() != stage:
        raise ValueError('Stage path differs from the immutable cutoff')
    if stage_seal.resolve() != cutoff.with_name('stage-source-sha256.json'):
        raise ValueError('Require the selected cutoff full-file seal')
    seal = json.loads(stage_seal.read_text())
    if files(stage) != seal:
        raise ValueError('Applied stage differs from the cutoff full-file seal')
    clean_official(official, source['horizon'])
    if digest(cutoff.with_name('plutolib.hnmod').read_bytes()) != source['archive_sha256']:
        raise ValueError('Archive differs from the selected source cutoff')
    if (stage / '.horizon-revision').read_text().strip() != source['horizon']:
        raise ValueError('Applied stage uses a different official revision')
    applied = json.loads((stage / '.horizon-mixins-stage.json').read_text())
    for name, sha in applied['inputs'].items():
        data = (source['horizon'] + '\n').encode() if name == '.horizon-revision' else (official / name).read_bytes()
        if digest(data) != sha:
            raise ValueError(f'Stage original source differs from the official checkout: {name}')
    guard.check(applied, official)
    entries = [c for c in applied['crates'] if c['package'] == 'pluto-inventory-api']
    if len(entries) != 1:
        raise ValueError('Exactly one stage-packaged inventory API is required')
    api = stage / entries[0]['path']; check_hashes(api, entries[0]['files'])
    check_hashes(ROOT / 'mixins/pluto_native_creative/inventory-api', entries[0]['files'])
    if output.exists():
        raise ValueError('Fixture output must be new')
    output.mkdir(parents=True)
    declarations = ['pub use pluto_inventory_api as pluto_inventory;']
    modules = {m['module_id']: m for m in applied['modules']}
    for id in ('stack-storage', 'stack-snapshot'):
        module = modules['pluto_native_creative-' + id]
        filename = id + '.rs'; (output / filename).write_text(module['source'])
        declarations.append(f'#[path={json.dumps(filename)}] pub mod {module["name"]};')
    (output / 'tests.rs').write_bytes((ROOT / 'tests/native/stack_snapshot.rs').read_bytes())
    declarations.append('#[path="tests.rs"] mod boundary;')
    (output / 'lib.rs').write_text('\n'.join(declarations) + '\n')
    (output / 'Cargo.toml').write_text('[package]\nname="pluto-stack-snapshot-tests"\nversion="0.0.0"\nedition="2024"\n[workspace]\n[lib]\npath="lib.rs"\n[dependencies]\n'
        'pluto-inventory-api={path=' + json.dumps(str(api)) + '}\n'
        'horizon-items={path=' + json.dumps(str(stage / 'crates/horizon-items')) + '}\n'
        'horizon-world={path=' + json.dumps(str(stage / 'crates/horizon-world')) + '}\n')
    patches = tomllib.loads((stage / 'Cargo.toml').read_text()).get('patch', {}).get('crates-io', {})
    with (output / 'Cargo.toml').open('a') as manifest:
        manifest.write('\n[patch.crates-io]\n')
        for name, entry in patches.items():
            if set(entry) != {'path'}:
                raise ValueError('Only exact staged path patches are supported')
            path = (stage / entry['path']).resolve()
            if not path.is_relative_to(stage):
                raise ValueError('Patch escapes approved stage')
            manifest.write(json.dumps(name) + '={path=' + json.dumps(str(path)) + '}\n')
    evidence = {'cutoff': source, 'official': str(official), 'stage': str(stage), 'api': str(api),
                'official_lock_sha256': digest((official / 'Cargo.lock').read_bytes()),
                'stage_files_sha256': seal, 'fixture_files_sha256': files(output),
                'scope': 'Actual applied native Wire reader and additive comparator only; no consumer adoption or live write authority.'}
    (output / 'Cargo.lock').write_bytes((official / 'Cargo.lock').read_bytes())
    dependencies(output, official, stage, api, locked=False)
    (output / 'fixture-sources.json').write_text(json.dumps(evidence, indent=2) + '\n')
    verify(output)
    print(output)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--horizon-source', type=Path)
    parser.add_argument('--staged-source', type=Path)
    parser.add_argument('--cutoff', type=Path)
    parser.add_argument('--stage-seal', type=Path)
    parser.add_argument('--test-output', type=Path)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--verify', action='store_true')
    args = parser.parse_args()
    if args.test_output:
        require_tests(args.test_output.read_text())
        print('All eight required real native boundary tests passed')
    elif args.verify:
        verify(args.output.resolve())
        print('Exact applied items/API/source/lock identities verified')
    elif any(p is None for p in (args.horizon_source, args.staged_source, args.cutoff, args.stage_seal)):
        parser.error('--horizon-source, --staged-source, --cutoff and --stage-seal are required')
    else:
        prepare(args.horizon_source, args.staged_source, args.cutoff, args.stage_seal, args.output)
