#!/usr/bin/env python3
"""Actual complete tank modules plus frozen complete legacy method bodies."""
from fixture_pluto_dependency import add_inventory_api, lock_fixture_to_host, run_locked_fixture, verify_fixture_lock
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
receipt = json.loads((rust / 'tank_compound_legacy.json').read_text())
files = [mod / ('source-' + name + '.rs') for name in
         ('casting-tank', 'smeltery-tank', 'fluid-container', 'smeltery-fuel', 'melter-state', 'fluid-display')]
files += [rust / name for name in ('pluto_storage_fixture.rs', 'pluto_potion_fixture.rs', 'tank_compound.rs',
    'tank_compound_smeltery_tank_legacy.rs', 'tank_compound_casting_tank_legacy.rs', 'tank_compound_legacy.json')]
files += [native / name for name in ('stack-storage.rs', 'nbt-storage.rs', 'stack-cells.rs', 'stack-fluid.rs', 'potion-facts.rs')]
files += list((native / 'inventory-api').rglob('*.rs')) + [native / 'inventory-api/Cargo.toml']
seal = {str(p.relative_to(root)): hashfile(p) for p in files}
for entry in receipt['files'].values():
    if hashfile(root / entry['oracle_path']) != entry['oracle_sha256']:
        raise ValueError('Immutable tank oracle changed')
with tempfile.TemporaryDirectory(prefix='tank-compound-') as temp:
    work = Path(temp); quote = lambda p: json.dumps(str(p)); manifest = work / 'Cargo.toml'
    manifest.write_text(f'''[package]
name = "tank-compound-fixture"
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
    source = f'''include!({quote(rust / 'pluto_storage_fixture.rs')});
include!({quote(rust / 'pluto_potion_fixture.rs')});
#[path = {quote(native / 'stack-fluid.rs')}] mod pluto_stack_fluid;
'''
    generated = {}
    for stem, ty in [('smeltery-tank', 'SmelteryTank'), ('casting-tank', 'CastingTank')]:
        original = mod / ('source-' + stem + '.rs'); production = original.read_bytes()
        entry = receipt['files'][str(original.relative_to(root))]
        oracle = (root / entry['oracle_path']).read_text()
        # Preserve the entire frozen method body; rename only its declaration
        # so it can coexist on the actual production type under cfg(test).
        oracle = oracle.replace('fn to_nbt_preserving(', 'fn legacy_to_nbt_preserving(', 1)
        actual = work / (stem + '-with-tests.rs')
        actual.write_bytes(production + f'\n#[cfg(test)]\nimpl {ty} {{\n{oracle}\n}}\n'.encode())
        if actual.read_bytes()[:len(production)] != production:
            raise ValueError('Actual module is not the exact source prefix')
        generated[actual] = (hashfile(actual), production)
        source += f'#[path = {quote(actual)}] mod tconstruct_source_{stem.replace("-", "_")};\n'
    for stem in ('fluid-container', 'smeltery-fuel', 'melter-state', 'fluid-display'):
        source += f'#[path = {quote(mod / ("source-" + stem + ".rs"))}] mod tconstruct_source_{stem.replace("-", "_")};\n'
    source += f'#[cfg(test)] mod tank_compound_tests {{ include!({quote(rust / "tank_compound.rs")}); }}\n'
    (work / 'lib.rs').write_text(source)
    lock_fixture_to_host(manifest, host, root)
    run_locked_fixture(manifest, ['tank_compound_tests::' + case for case in (
        'smeltery_complete_legacy_method_matches_owned_patch_matrix',
        'casting_complete_legacy_method_matches_raw_slot_and_alias_matrix',
        'unknown_utf16_and_float_bits_stay_on_exact_row_identity',
        'casting_rejects_root_and_nested_retained_wire_without_changing_inputs',
        'native_compound_has_last_value_and_sorted_exact_java_keys',
    )])
    verify_fixture_lock(manifest, host, root)
    for actual, (expected, production) in generated.items():
        if hashfile(actual) != expected or actual.read_bytes()[:len(production)] != production:
            raise ValueError('Generated module or exact prefix changed')
    if any(hashfile(root / p) != sha for p, sha in seal.items()):
        raise ValueError('Fixture/consumer/API source changed')
    if subprocess.check_output(['git', '-C', str(host), 'status', '--porcelain', '--untracked-files=no'], text=True):
        raise ValueError('Official tracked source changed')
    print(json.dumps({'source_sha256': seal, 'actual_module_prefix_sha256': {
        p.name: hashlib.sha256(source).hexdigest() for p, (_, source) in generated.items()},
        'official_revision': subprocess.check_output(['git', '-C', str(host), 'rev-parse', 'HEAD'], text=True).strip()}, indent=2))
