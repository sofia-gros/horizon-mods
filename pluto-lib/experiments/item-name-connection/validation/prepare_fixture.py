"""Prepare bounded tests from actual official Mixins-applied source.

Never runs plan/apply/build/test. --resolve is a separate explicit metadata
operation. The CI owner must capture the authorized apply result and seal its
output before calling this preparer. A stage marker alone is not proof of apply.
"""
import argparse
import hashlib
import importlib.util
import json
from pathlib import Path
import re
import shutil
import subprocess
import tomllib

HERE = Path(__file__).resolve().parent
PIN = '624c4f6265c02296fbe8a80f029dd0d987fb94c0'
NAME = 'pluto-item-name-removal-tests'
SPEC = importlib.util.spec_from_file_location('item_name_guard', HERE / 'check_plan.py')
guard = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(guard)
MODIFIED = {'crates/horizon-client/src/client_lib.rs', guard.GUEST, guard.SYNC}
MARKER = '.horizon-mixins-stage.json'

def sha(data): return hashlib.sha256(data).hexdigest()

def run(args, cwd=None):
    return subprocess.check_output(args, cwd=cwd, text=True, timeout=60)

def clean_official(host):
    remote = run(['git', '-C', str(host), 'config', '--get', 'remote.origin.url']).strip()
    if remote not in {'https://github.com/horizon-rust/horizon.git', 'https://github.com/horizon-rust/horizon'}:
        raise ValueError('Not the authorized official repository')
    if run(['git', '-C', str(host), 'rev-parse', 'HEAD']).strip() != PIN:
        raise ValueError('Official revision differs')
    if run(['git', '-C', str(host), 'status', '--porcelain', '--untracked-files=all', '--ignored=matching']).strip():
        raise ValueError('Official source has tracked, untracked or ignored changes')

def files(root):
    result = {}
    for path in root.rglob('*'):
        if path.is_symlink(): raise ValueError('Symlink in generated input')
        if path.is_file(): result[path.relative_to(root).as_posix()] = sha(path.read_bytes())
        elif not path.is_dir(): raise ValueError('Special file in generated input')
    return result

def section(text, start, end):
    if text.count(start) != 1 or text.count(end) != 1:
        raise ValueError('Native source extraction boundary is not unique')
    a, b = text.index(start), text.index(end)
    if b <= a: raise ValueError('Native extraction boundary order changed')
    return text[a:b]

def final_module(text, name):
    start = 'pub(crate) mod ' + name + ' {'
    if text.count(start) != 1: raise ValueError('Applied module is missing/ambiguous')
    # This isolated plan appends one support module to each selected file.
    # No later module is permitted; the actual file tail is kept byte-for-byte.
    return text[text.index(start):]

def source_evidence(host, stage, plan_path):
    clean_official(host)
    plan = json.loads(plan_path.read_text())
    guard.check(plan, host)
    if json.loads((stage / MARKER).read_text()) != plan:
        raise ValueError('Actual stage plan differs from checked plan')
    if len(plan['injections']) != 3 or len(plan.get('modules', [])) != 2:
        raise ValueError('Bounded fixture requires only the isolated candidate')
    if any(plan.get(key) for key in ('bridges', 'states', 'crates', 'data', 'post_build')):
        raise ValueError('Additional applied capabilities are outside this fixture')
    actual = files(stage)
    if set(actual) != set(plan['inputs']) | {MARKER}:
        raise ValueError('Applied stage file-set differs from official inputs')
    changed = set()
    for relative, digest in plan['inputs'].items():
        if sha((host / relative).read_bytes()) != digest:
            raise ValueError('Official input changed: ' + relative)
        if actual[relative] != digest: changed.add(relative)
    if changed != MODIFIED:
        raise ValueError('Applied source changed outside the exact three files')
    return {'plan_sha256': sha(plan_path.read_bytes()), 'official_inputs': plan['inputs'],
            'stage_files': actual, 'modified_files': sorted(changed)}

def prepare(host, stage, plan_path, output):
    host, stage, plan_path = host.resolve(), stage.resolve(), plan_path.resolve()
    evidence = source_evidence(host, stage, plan_path)
    expected = json.loads((HERE / 'fixture-inputs.json').read_text())
    for relative, digest in expected['source_sha256'].items():
        if sha((HERE / relative).read_bytes()) != digest:
            raise ValueError('Frozen proof input changed: ' + relative)
    output.mkdir(parents=True, exist_ok=False)
    fragments = []
    def emit(text, destination, source, boundaries):
        (output / destination).write_text(text)
        fragments.append({'source': source, 'boundaries': boundaries,
                          'destination': destination, 'sha256': sha(text.encode())})
    guest_bounds = ('fn network_explosion(', '/// `Player.isWithinBlockInteractionRange`')
    sync_bounds = ('use crate::items::ItemStack;', '#[cfg(test)]\nmod tests')
    for prefix, root in [('', stage), ('native_', host)]:
        guest_text = (root / guard.GUEST).read_text()
        guest = section(guest_text, *guest_bounds)
        if not prefix:
            guest += final_module(guest_text, 'pluto_name_entry_gate')
        emit('use crate::items::ItemStack;\nuse horizon_protocol::item::Stack;\n' + guest,
             prefix + 'guest.rs', str(root / guard.GUEST), list(guest_bounds))
        emit(section((root / guard.SYNC).read_text(), *sync_bounds), prefix + 'item_sync.rs',
             str(root / guard.SYNC), list(sync_bounds))
    emit(final_module((stage / 'crates/horizon-client/src/client_lib.rs').read_text(), 'pluto_name_removal'),
         'name_module.rs', str(stage / 'crates/horizon-client/src/client_lib.rs'), ['final pluto_name_removal module'])
    dye_path = host / 'crates/horizon-client-render/src/block_entity_render.rs'
    dye = section(dye_path.read_text(), 'pub const DYES:', 'fn dye_prefix(')
    emit(dye, 'dyes.rs', str(dye_path), ['pub const DYES:', 'fn dye_prefix('])
    shutil.copyfile(HERE / 'codec_tests.rs', output / 'codec_tests.rs')
    shutil.copyfile(host / 'Cargo.lock', output / 'Cargo.lock')
    (output / 'lib.rs').write_text('''pub use horizon_items::{items, item_data};
pub mod particles { pub use horizon_items::firework::FireworkExplosion; }
pub mod save { pub use horizon_client_core::nbt_value::{float, number, text}; }
#[path="dyes.rs"] pub mod block_entity_render;
#[path="guest.rs"] mod guest;
#[path="native_guest.rs"] mod native_guest;
#[path="item_sync.rs"] mod item_sync;
#[path="native_item_sync.rs"] mod native_sync;
include!("name_module.rs");
#[cfg(test)] mod codec_tests;
''')
    cargo = '[package]\nname="' + NAME + '"\nversion="0.0.0"\nedition="2024"\n[workspace]\n[lib]\npath="lib.rs"\n[dependencies]\nlog="0.4"\n'
    for package in ('horizon-items', 'horizon-client-core', 'horizon-protocol', 'horizon-world'):
        cargo += package + '={path=' + json.dumps(str(host / 'crates' / package)) + '}\n'
    (output / 'Cargo.toml').write_text(cargo)
    evidence.update({'official': PIN, 'host': str(host), 'stage': str(stage), 'plan': str(plan_path),
                     'fragments': fragments, 'source_files': files(output), 'resolved': False,
                     'expected_tests': expected['expected_tests'], 'result_groups': expected['result_groups'],
                     'proof_scope': 'actual applied native function regions and codecs; not installed Game/network execution'})
    (output / 'fixture-sources.json').write_text(json.dumps(evidence, indent=2) + '\n')
    return evidence

def verify(output, resolved=False):
    evidence = json.loads((output / 'fixture-sources.json').read_text())
    fresh = source_evidence(Path(evidence['host']), Path(evidence['stage']), Path(evidence['plan']))
    for key, value in fresh.items():
        if evidence[key] != value: raise ValueError('Applied input drift: ' + key)
    actual = files(output); actual.pop('fixture-sources.json')
    if actual != evidence['source_files']: raise ValueError('Fixture source/lock/file-set drift')
    if resolved and not evidence['resolved']: raise ValueError('Dependency identity has not been resolved')
    return evidence

def resolve(output):
    evidence = verify(output)
    host = Path(evidence['host'])
    metadata = json.loads(run(['cargo', 'metadata', '--offline', '--format-version=1'], cwd=output))
    identity = lambda p: (p['name'], p['version'], p.get('source'), p.get('checksum'))
    allowed = {identity(p) for p in tomllib.loads((host / 'Cargo.lock').read_text())['package']}
    actual = {identity(p) for p in tomllib.loads((output / 'Cargo.lock').read_text())['package']}
    local = (NAME, '0.0.0', None, None)
    if actual - allowed - {local} or local not in actual: raise ValueError('Dependency lock identity escaped official lock')
    for package in metadata['packages']:
        path = Path(package['manifest_path']).resolve()
        if package['name'] == NAME:
            if path != (output / 'Cargo.toml').resolve() or package['source'] is not None: raise ValueError('Wrong fixture identity')
        elif package['source'] is None and not path.is_relative_to(host / 'crates'):
            raise ValueError('Dependency escaped official source')
    for name in ('horizon-items', 'horizon-client-core', 'horizon-protocol', 'horizon-world'):
        matches = [p for p in metadata['packages'] if p['name'] == name]
        if len(matches) != 1 or Path(matches[0]['manifest_path']).resolve() != host / 'crates' / name / 'Cargo.toml':
            raise ValueError('Expected one actual native dependency: ' + name)
    evidence['source_files']['Cargo.lock'] = sha((output / 'Cargo.lock').read_bytes())
    evidence['resolved'] = True
    evidence['dependencies'] = [{'name': p['name'], 'manifest_path': p['manifest_path'], 'source': p['source']} for p in metadata['packages']]
    (output / 'fixture-sources.json').write_text(json.dumps(evidence, indent=2) + '\n')
    verify(output, resolved=True)
    return evidence

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--official', type=Path)
    parser.add_argument('--stage', type=Path)
    parser.add_argument('--plan', type=Path)
    parser.add_argument('--output', type=Path, required=True)
    group = parser.add_mutually_exclusive_group()
    group.add_argument('--verify', action='store_true')
    group.add_argument('--resolve', action='store_true')
    args = parser.parse_args()
    if args.verify: result = verify(args.output, resolved=True)
    elif args.resolve: result = resolve(args.output)
    else:
        if not all((args.official, args.stage, args.plan)): parser.error('official, stage and plan are required')
        result = prepare(args.official, args.stage, args.plan, args.output)
    print(json.dumps({'prepared': True, 'resolved': result['resolved'], 'expected_tests': result['expected_tests']}))

if __name__ == '__main__': main()
