#!/usr/bin/env python3
"""Compare additive plain-cell helpers to real unchanged TConstruct wrappers."""
from fixture_pluto_dependency import add_inventory_api, lock_fixture_to_host, run_locked_fixture, verify_fixture_lock
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile

root = Path(__file__).resolve().parents[2]
host = Path(os.environ['HORIZON_HOST']).resolve()
paths = [
    'tconstruct/mixins/tconstruct/source-tinker-chests.rs',
    'tconstruct/mixins/tconstruct/source-tinker-chest-click.rs',
    'tconstruct/mixins/tconstruct/source-side-chest-click.rs',
    'tconstruct/mixins/tconstruct/source-chest-guest-click.rs',
    'tconstruct/mixins/tconstruct/source-crafting-lan.rs',
    'pluto-lib/mixins/pluto_native_creative/stack-cells.rs',
    'tconstruct/tests/rust/plain_stack_cells.rs',
    'tconstruct/tests/rust/plain_stack_cells_legacy.rs',
    'tconstruct/tests/rust/plain_stack_cells_legacy.json',
]
seal = {p: hashlib.sha256((root / p).read_bytes()).hexdigest() for p in paths}
with tempfile.TemporaryDirectory(prefix='plain-stack-cells-') as temp:
    work = Path(temp)
    quote = lambda p: json.dumps(str(p))
    manifest = work / 'Cargo.toml'
    manifest.write_text(f'''[package]
name = "plain-stack-cells-fixture"
version = "0.1.0"
edition = "2024"
[dependencies]
horizon-items = {{ path = {quote(host / 'crates/horizon-items')} }}
horizon-world = {{ path = {quote(host / 'crates/horizon-world')} }}
[lib]
path = "lib.rs"
''')
    add_inventory_api(manifest, root)
    oracle = json.loads((root / 'tconstruct/tests/rust/plain_stack_cells_legacy.json').read_text())
    if hashlib.sha256((root / 'tconstruct/tests/rust/plain_stack_cells_legacy.rs').read_bytes()).hexdigest() != oracle['oracle_sha256']:
        raise ValueError('Frozen legacy oracle differs from its receipt')
    (work / 'lib.rs').write_text(f'''
include!({quote(root / 'tconstruct/tests/rust/pluto_storage_fixture.rs')});
#[path = {quote(root / 'tconstruct/mixins/tconstruct/chest-inventory-api-lib.rs')}]
mod tconstruct_inventory_api;
#[path = {quote(root / 'tconstruct/mixins/tconstruct/source-tinker-chests.rs')}]
mod consumer;
#[path = {quote(root / 'tconstruct/tests/rust/plain_stack_cells_legacy.rs')}]
mod legacy;
use pluto_stack_cells as cells;
#[cfg(test)]
#[path = {quote(root / 'tconstruct/tests/rust/plain_stack_cells.rs')}]
mod differential;
''')
    lock_fixture_to_host(manifest, host, root)
    run_locked_fixture(manifest, [f'differential::{name}' for name in [
        'pickup_valid_inputs_match_actual_legacy_wrapper',
        'take_valid_inputs_match_actual_legacy_wrapper',
        'ordered_collection_matches_actual_legacy_for_duplicates_padding_and_two_passes',
        'invalid_pickup_is_rejected_before_mutation_or_policy',
        'invalid_collection_rejects_before_any_partial_merge',
        'caller_merge_policy_controls_opaque_identity_and_keeps_carried_identity',
        'consumer_wrappers_reject_malformed_inputs_without_changing_values',
        'chest_slot_delegation_preserves_admission_limits_and_empty_cursor_bypass',
        'chest_slot_rejects_invalid_shape_and_malformed_values_without_mutation',
    ]])
    verify_fixture_lock(manifest, host, root)
    if subprocess.check_output(['git', '-C', str(host), 'status', '--porcelain', '--untracked-files=no'], text=True):
        raise ValueError('Official host tracked source changed during fixture')
    if any(hashlib.sha256((root / p).read_bytes()).hexdigest() != digest for p, digest in seal.items()):
        raise ValueError('Fixture or caller source changed during execution')
    print(json.dumps({'source_sha256': seal, 'official_revision': subprocess.check_output(['git', '-C', str(host), 'rev-parse', 'HEAD'], text=True).strip()}, indent=2))
