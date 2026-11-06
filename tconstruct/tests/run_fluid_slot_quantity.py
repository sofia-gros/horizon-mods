#!/usr/bin/env python3
"""Exact actual fluid-container module plus immutable old-body differential tests."""
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
files = [mod / ('source-' + name + '.rs') for name in
         ('fluid-container', 'smeltery-fuel', 'melter-state', 'smeltery-tank', 'fluid-display', 'smeltery-click', 'smeltery-guest-guard')]
files += [root / 'tconstruct/tests/rust' / name for name in
          ('pluto_storage_fixture.rs', 'pluto_potion_fixture.rs', 'fluid_slot_quantity.rs', 'fluid_slot_quantity_legacy.rs', 'fluid_slot_quantity_legacy.json')]
files += [native / name for name in ('stack-storage.rs', 'nbt-storage.rs', 'stack-cells.rs', 'stack-fluid.rs', 'potion-facts.rs')]
hashfile = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
seal = {str(p.relative_to(root)): hashfile(p) for p in files}
receipt = json.loads((root / 'tconstruct/tests/rust/fluid_slot_quantity_legacy.json').read_text())
if hashfile(root / 'tconstruct/tests/rust/fluid_slot_quantity_legacy.rs') != receipt['oracle_sha256']:
    raise ValueError('The immutable fluid slot oracle changed')
with tempfile.TemporaryDirectory(prefix='fluid-slot-quantity-') as temp:
    work = Path(temp)
    quote = lambda p: json.dumps(str(p))
    manifest = work / 'Cargo.toml'
    manifest.write_text(f'''[package]
name = "fluid-slot-quantity-fixture"
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
    production = (mod / 'source-fluid-container.rs').read_bytes()
    extension = f'''
#[cfg(test)]
mod quantity_legacy {{
    use super::{{ItemStack, accepts_with_color, bottle_fluid}};
    include!({quote(root / 'tconstruct/tests/rust/fluid_slot_quantity_legacy.rs')});
}}
#[cfg(test)]
mod quantity_tests {{
    include!({quote(root / 'tconstruct/tests/rust/fluid_slot_quantity.rs')});
}}
'''.encode()
    actual = work / 'fluid-container-with-tests.rs'
    actual.write_bytes(production + extension)
    if actual.read_bytes()[:len(production)] != production:
        raise ValueError('Actual module bytes are not the exact source prefix')
    expected_generated = hashfile(actual)
    source = f'''include!({quote(root / 'tconstruct/tests/rust/pluto_storage_fixture.rs')});
include!({quote(root / 'tconstruct/tests/rust/pluto_potion_fixture.rs')});
#[path = {quote(native / 'stack-fluid.rs')}]
mod pluto_stack_fluid;
#[path = {quote(actual)}]
mod tconstruct_source_fluid_container;
'''
    for name in ('smeltery-fuel', 'melter-state', 'smeltery-tank', 'fluid-display'):
        module = 'tconstruct_source_' + name.replace('-', '_')
        source += f'#[path = {quote(mod / ("source-" + name + ".rs"))}]\nmod {module};\n'
    (work / 'lib.rs').write_text(source)
    lock_fixture_to_host(manifest, host, root)
    run_locked_fixture(manifest, ['tconstruct_source_fluid_container::quantity_tests::' + case for case in (
        'clicks_match_frozen_body_values_and_known_fluid_trace',
        'quick_input_matches_frozen_body_values_and_known_fluid_trace',
        'empty_cursor_extraction_and_missing_input_remain_legacy_operations',
        'every_admission_family_rejects_zero_incoming_before_fluid_queries',
        'exact_legacy_component_policy_keeps_opaque_receiving_value',
    )])
    verify_fixture_lock(manifest, host, root)
    if hashfile(actual) != expected_generated or actual.read_bytes()[:len(production)] != production:
        raise ValueError('Generated actual module or exact prefix changed')
    if any(hashfile(root / p) != sha for p, sha in seal.items()):
        raise ValueError('Fixture/consumer source changed')
    if subprocess.check_output(['git', '-C', str(host), 'status', '--porcelain', '--untracked-files=no'], text=True):
        raise ValueError('Official tracked source changed')
    print(json.dumps({'source_sha256': seal, 'actual_module_prefix_sha256': hashlib.sha256(production).hexdigest(),
                      'official_revision': subprocess.check_output(['git', '-C', str(host), 'rev-parse', 'HEAD'], text=True).strip()}, indent=2))
