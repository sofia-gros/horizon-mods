#!/usr/bin/env python3
"""Exact native state modules and three complete old/current functions.

Drain's Game/world are the existing fixture, not native Game authority.
Run only in a granted, disk-guarded native slot. Path/lock identity alone does
not promise Cargo cache reuse: feature/dependency fingerprints also matter.
"""
from fixture_pluto_dependency import add_inventory_api, lock_fixture_to_host, verify_fixture_lock
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile

root = Path(__file__).resolve().parents[2]
host = Path(os.environ['HORIZON_HOST']).resolve()
mod = root / 'tconstruct/mixins/tconstruct'
native = root / 'pluto-lib/mixins/pluto_native_creative'
rust = root / 'tconstruct/tests/rust'
hashfile = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
meta = json.loads((rust / 'outer_tank_legacy.json').read_text())

def body(source, marker):
    start = source.index(marker); end = source.index('{', start) + 1; depth = 1
    while depth:
        depth += (source[end] == '{') - (source[end] == '}'); end += 1
    return source[start:end]

files = [mod / ('source-' + name + '.rs') for name in ('fluid-cannon', 'machine-runtime',
    'smeltery-tank', 'smeltery-state', 'fluid-container', 'smeltery-fuel', 'melter-state', 'fluid-display')]
files += [rust / name for name in ('pluto_storage_fixture.rs', 'pluto_potion_fixture.rs',
    'drain_container_fixture.rs', 'outer_tank_patch.rs', 'outer_tank_legacy.json')]
files += [root / entry['oracle_path'] for entry in meta['methods'].values()]
files += [native / name for name in ('stack-storage.rs', 'nbt-storage.rs', 'stack-cells.rs', 'stack-fluid.rs', 'potion-facts.rs')]
files += list((native / 'inventory-api').rglob('*.rs')) + [native / 'inventory-api/Cargo.toml']
seal = {str(p.relative_to(root)): hashfile(p) for p in files}
for entry in meta['methods'].values():
    if hashfile(root / entry['oracle_path']) != entry['oracle_sha256']:
        raise ValueError('Immutable outer tank oracle changed')
    actual = body((root / entry['path']).read_text(), entry['declaration'])
    if hashlib.sha256(actual.encode()).hexdigest() != entry['adopted_method_sha256']:
        raise ValueError('Current complete method differs from frozen source')
with tempfile.TemporaryDirectory(prefix='outer-tank-patch-') as temp:
    work = Path(temp); quote = lambda p: json.dumps(str(p)); manifest = work / 'Cargo.toml'
    manifest.write_text(f'''[package]
name = "outer-tank-patch-fixture"
version = "0.1.0"
edition = "2024"
[dependencies]
horizon-items = {{ path = {quote(host / 'crates/horizon-items')} }}
horizon-world = {{ path = {quote(host / 'crates/horizon-world')} }}
horizon-client-core = {{ path = {quote(host / 'crates/horizon-client-core')} }}
[lib]
path = "lib.rs"
''')
    add_inventory_api(manifest, root)
    production = (mod / 'source-fluid-cannon.rs').read_bytes()
    oracle = (root / meta['methods']['cannon']['oracle_path']).read_text()
    oracle = oracle.replace('fn tank_to_nbt_preserving(', 'fn legacy_tank_to_nbt_preserving(', 1)
    actual = work / 'cannon-with-oracle.rs'
    actual.write_bytes(production + f'\n#[cfg(test)]\nimpl Cannon {{\n{oracle}\n}}\n'.encode())
    if actual.read_bytes()[:len(production)] != production:
        raise ValueError('Actual Cannon module is not the exact production prefix')
    actual_sha = hashfile(actual)
    # Keep the existing bounded world/structure/callback fixture unchanged.
    scaffold = (rust / 'drain_container_fixture.rs').read_text()
    scaffold = scaffold[:scaffold.index('#[test]')]
    runtime = (mod / 'source-machine-runtime.rs').read_text()
    for key, token in [('drain', '// __PRODUCTION_ROUTE__'), ('controller', '// __PRODUCTION_PATCH__')]:
        entry = meta['methods'][key]
        current = body(runtime, entry['declaration'])
        old = (root / entry['oracle_path']).read_text()
        name = 'interact_drain' if key == 'drain' else 'patch_smeltery_controller_tank'
        old = old.replace('fn ' + name + '(', 'fn legacy_' + name + '(', 1)
        scaffold = scaffold.replace(token, current + '\n' + old)
    # Put tests in the same module so private helper visibility is unchanged.
    scaffold = scaffold.replace('\n}\nfn fixture(', f'\n#[cfg(test)] mod outer_tank_tests {{ include!({quote(rust / "outer_tank_patch.rs")}); }}\n}}\nfn fixture(', 1)
    scaffold = scaffold.replace('__MIXINS__', str(mod)).replace('__PLUTO__', str(native / 'stack-fluid.rs'))
    scaffold += f'\n#[path = {quote(actual)}] mod tconstruct_source_fluid_cannon;\n'
    lib = work / 'lib.rs'; lib.write_text(scaffold); lib_sha = hashfile(lib)
    lock_fixture_to_host(manifest, host, root)
    required = ['runtime::outer_tank_tests::' + name for name in (
        'cannon_complete_old_method_matches_raw_and_state_matrix',
        'cannon_retained_wire_and_opaque_item_contracts_match',
        'controller_patch_matches_unknown_and_malformed_values',
        'drain_complete_route_matches_saved_and_published_results',
    )]
    result = subprocess.run(['cargo', 'test', '--lib', '--locked', '--offline', '--manifest-path', str(manifest)],
        text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    print(result.stdout)
    result.check_returncode()
    for case in required:
        if f'test {case} ... ok' not in result.stdout.splitlines():
            raise ValueError(f'Required actual boundary case did not pass: {case}')
    verify_fixture_lock(manifest, host, root)
    if hashfile(actual) != actual_sha or hashfile(lib) != lib_sha or actual.read_bytes()[:len(production)] != production:
        raise ValueError('Generated actual source changed')
    if any(hashfile(root / p) != sha for p, sha in seal.items()):
        raise ValueError('Consumer/fixture/API source changed')
    if subprocess.check_output(['git', '-C', str(host), 'status', '--porcelain', '--untracked-files=no'], text=True):
        raise ValueError('Official tracked source changed')
    print(json.dumps({'source_sha256': seal, 'actual_cannon_prefix_sha256': hashlib.sha256(production).hexdigest(),
        'official_revision': subprocess.check_output(['git', '-C', str(host), 'rev-parse', 'HEAD'], text=True).strip(),
        'scope': 'Real native ItemStack/NBT and actual complete functions; existing mock Game/world, no native authority proof.'}, indent=2))
