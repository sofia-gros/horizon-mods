#!/usr/bin/env python3
"""Compare native canonical decoding and actual Smeltery callers with old rules."""
from fixture_pluto_dependency import add_inventory_api, lock_fixture_to_host
import json
import os
from pathlib import Path
import subprocess
import tempfile

root = Path(__file__).resolve().parents[2]
host = Path(os.environ['HORIZON_HOST']).resolve()
with tempfile.TemporaryDirectory(prefix='item-canonical-decode-') as temp:
    work = Path(temp)
    (work / 'Cargo.toml').write_text(f'''[package]
name = "item-canonical-decode-fixture"
version = "0.1.0"
edition = "2024"
[dependencies]
horizon-items = {{ path = {json.dumps(str(host / 'crates/horizon-items'))} }}
horizon-world = {{ path = {json.dumps(str(host / 'crates/horizon-world'))} }}
horizon-client-core = {{ path = {json.dumps(str(host / 'crates/horizon-client-core'))} }}
[lib]
path = "lib.rs"
''')
    add_inventory_api(work / 'Cargo.toml', root)
    fixture = root / 'tconstruct/tests/rust/item_canonical_decode.rs'
    (work / 'lib.rs').write_text(f'include!({json.dumps(str(fixture))});\n')
    lock_fixture_to_host(work / 'Cargo.toml', host, root)
    result = subprocess.run(['cargo', 'test', '--locked', '--offline', '--manifest-path', str(work / 'Cargo.toml')],
                            text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    print(result.stdout)
    result.check_returncode()
    for case in (
        'native_decode_matches_previous_tag_roundtrip_matrix',
        'canonicality_is_not_saved_or_runtime_snapshot_identity',
        'single_insert_matches_previous_whole_function_matrix',
        'inventory_patch_matches_previous_whole_function_matrix',
        'inventory_patch_rejects_nested_wire_but_keeps_existing_plain_policy',
        'owned_compound_patches_match_previous_writes_and_preserve_raw_extensions',
    ):
        if f'test canonical_tests::{case} ... ok' not in result.stdout.splitlines():
            raise SystemExit(f'Required canonical item test did not pass: {case}')
