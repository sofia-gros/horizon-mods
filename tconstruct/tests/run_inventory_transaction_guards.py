#!/usr/bin/env python3
"""Check production inventory guards with the pinned official host's real types."""
from fixture_pluto_dependency import add_inventory_api, inventory_api_identity
import json
import os
from pathlib import Path
import subprocess
import tempfile
import tomllib

root = Path(__file__).resolve().parents[2]
host = Path(os.environ["HORIZON_HOST"]).resolve()
expected = json.loads((root / "tconstruct/compatibility.json").read_text())["horizon_revision"]
actual = subprocess.check_output(["git", "-C", str(host), "rev-parse", "HEAD"], text=True).strip()
dirty = subprocess.check_output(["git", "-C", str(host), "status", "--porcelain", "--untracked-files=no"], text=True)
if actual != expected or dirty:
    raise SystemExit(f"HORIZON_HOST must have a clean tracked tree at {expected}")
source = (root / "tconstruct/mixins/tconstruct/source-chute-hopper.rs").read_text()
start = source.index("use horizon_client::pluto_stack_storage::encode_item_lossless as encode_moving_item;")
end = source.index("fn tconstruct_chute_transfer(", start)
fixture = (root / "tconstruct/tests/rust/inventory_transaction_guards.rs").read_text()
fixture = fixture.replace("// __PRODUCTION_GUARDS__", source[start:end])
fixture = fixture.replace("__STACK_STORAGE__", str(root / "pluto-lib/mixins/pluto_native_creative/stack-storage.rs"))
fixture = fixture.replace("__NBT_STORAGE__", str(root / "pluto-lib/mixins/pluto_native_creative/nbt-storage.rs"))
with tempfile.TemporaryDirectory(prefix="inventory-transaction-guards-") as temp:
    work = Path(temp)
    (work / "lib.rs").write_text(fixture)
    (work / "Cargo.toml").write_text(f'''[package]
name = "inventory-transaction-guards-fixture"
version = "0.1.0"
edition = "2024"
[dependencies]
horizon-items = {{path = {json.dumps(str(host / "crates/horizon-items"))}}}
horizon-world = {{path = {json.dumps(str(host / "crates/horizon-world"))}}}
[lib]
path = "lib.rs"
''')
    add_inventory_api(work / "Cargo.toml", root)
    host_lock = (host / "Cargo.lock").read_text()
    (work / "Cargo.lock").write_text(host_lock + '''
[[package]]
name = "inventory-transaction-guards-fixture"
version = "0.1.0"
dependencies = ["horizon-items", "horizon-world"]
''')
    command = ["--offline", "--manifest-path", str(work / "Cargo.toml")]
    subprocess.run(["cargo", "metadata", *command, "--format-version", "1"], check=True, stdout=subprocess.DEVNULL)
    def identity(package):
        return tuple(package.get(key) for key in ("name", "version", "source", "checksum"))
    allowed = {identity(package) for package in tomllib.loads(host_lock)["package"]}
    allowed.add(inventory_api_identity(root))
    unknown = [identity(package) for package in tomllib.loads((work / "Cargo.lock").read_text())["package"]
               if package["name"] != "inventory-transaction-guards-fixture" and identity(package) not in allowed]
    if unknown:
        raise SystemExit(f"Fixture dependencies differ from pinned host lock: {unknown}")
    result = subprocess.run(["cargo", "test", "--locked", *command],
                            text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    print(result.stdout)
    result.check_returncode()
    for case in ("official_stack_roundtrip_preserves_count_damage_and_components",
                 "retained_wire_is_rejected_even_when_public_patch_is_stale",
                 "production_snapshot_guard_rejects_count_component_and_raw_storage_changes",
                 "production_snapshot_guard_observes_retained_wire_changes",
                 "nested_retained_wire_rejected_in_every_native_stack_field",
                 "nested_wire_bytes_presence_and_usability_are_snapshot_identity",
                 "nested_wire_callback_change_in_unselected_slot_aborts_transfer",
                 "ordinary_nested_stacks_still_roundtrip_and_structure_changes_reject",
                 "wire_checks_reach_nested_mixed_stack_trees",
                 "bundle_selection_is_snapshot_state_but_not_saved_item_identity"):
        if f"test host_tests::{case} ... ok" not in result.stdout.splitlines():
            raise SystemExit(f"Required production boundary test did not run: {case}")

    for case in ("ordered_updates_set_remove_and_preserve_unowned_fields",
                 "last_update_can_remove_a_new_field",
                 "unknown_utf16_keys_nested_values_and_float_bits_are_untouched",
                 "malformed_original_does_not_consume_update_iterator",
                 "owned_patch_removes_missing_owned_fields_and_ignores_other_proposals",
                 "owned_patch_rejects_malformed_values_even_with_no_owned_keys"):
        if f"test pluto_nbt_storage::tests::{case} ... ok" not in result.stdout.splitlines():
            raise SystemExit(f"Required native storage patch test did not run: {case}")
