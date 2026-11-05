#!/usr/bin/env python3
"""Run production Drain interaction with real Horizon state types and mocked world.

HORIZON_HOST must name the pinned host checkout. Cargo dependencies are real;
world storage, structure detection and inventory access are fixtures.
"""
import os, pathlib, subprocess, tempfile, tomllib
root = pathlib.Path(__file__).resolve().parents[1]
host = pathlib.Path(os.environ["HORIZON_HOST"]).resolve()
expected = "0c7b5b61a5e746d621afffba919b12aeccf9c0fa"
actual = subprocess.check_output(["git", "-C", str(host), "rev-parse", "HEAD"], text=True).strip()
dirty = subprocess.check_output(["git", "-C", str(host), "status", "--porcelain", "--untracked-files=no"], text=True)
if actual != expected or dirty:
    raise SystemExit(f"HORIZON_HOST must have a clean tracked tree at {expected}")
source = (root / "mixins/tconstruct/source-machine-runtime.rs").read_text()
route = source[source.index("pub(super) fn interact_drain("):source.index("pub(super) fn ensure(")]
held_fuel = source[source.index("pub(super) fn interact_fuel_tank("):source.index("\npub(super) fn casting_tank_at(")]
patch = source[source.index("fn patch_smeltery_controller_tank("):source.index("\nfn validated_drain(")]
direct_source = (root / "mixins/tconstruct/source-fluid-direct.rs").read_text()
direct = direct_source[direct_source.index("pub(super) fn local("):direct_source.index("pub(super) fn guest(")]
melter_exchange = direct_source[direct_source.index("fn melter_exchange("):direct_source.index("pub(super) fn melter_local(")].replace("fn melter_exchange(", "pub(super) fn melter_exchange(", 1)
fuel_exchange = direct_source[direct_source.index("fn fuel_exchange("):direct_source.index("pub(super) fn fuel_local(")].replace("fn fuel_exchange(", "pub(super) fn fuel_exchange(", 1)
use_source = (root / "mixins/tconstruct/source-machine-use.rs").read_text()
melter_arm = use_source.split('"tconstruct:seared_melter" => {', 1)[1].split('"tconstruct:copper_gauge" =>', 1)[0].rsplit('},', 1)[0]
melter_arm = melter_arm.replace("crate::game::", "crate::")
tank_drop = (root / "mixins/tconstruct/source-smeltery-tank-drop.rs").read_text()
tank_drop = tank_drop.replace("fn smeltery_tank_drop(", "pub(super) fn smeltery_tank_drop(", 1).replace("crate::game::spill::Pending", "Pending").replace("crate::game::tconstruct_source_smeltery_fuel", "crate::tconstruct_source_smeltery_fuel").replace("horizon_client::items::ItemStack", "horizon_items::items::ItemStack")
kind = source[source.index("pub(super) fn kind("):source.index("\nfn initial(")]
template = (root / "tests/rust/drain_container_fixture.rs").read_text().replace("// __PRODUCTION_DIRECT_LOCAL__", direct).replace("// __PRODUCTION_MELTER_EXCHANGE__", melter_exchange).replace("// __PRODUCTION_FUEL_EXCHANGE__", fuel_exchange).replace("// __PRODUCTION_KIND__", kind).replace("// __PRODUCTION_PATCH__", patch).replace("// __PRODUCTION_HELD_FUEL__", held_fuel).replace("// __PRODUCTION_MELTER_ARM__", melter_arm).replace("// __PRODUCTION_TANK_DROP__", tank_drop)
template = template.replace("__MIXINS__", str(root / "mixins/tconstruct")).replace("__PLUTO__",str(root.parent / "pluto-lib/mixins/pluto_native_creative/stack-fluid.rs")).replace("// __PRODUCTION_ROUTE__", route)
with tempfile.TemporaryDirectory(prefix="drain-container-") as tmp:
    path = pathlib.Path(tmp)
    (path / "src").mkdir()
    (path / "src/lib.rs").write_text(template)
    (path / "Cargo.toml").write_text('[package]\nname="drain-container-fixture"\nversion="0.1.0"\nedition="2024"\n[dependencies]\nhorizon-items={path="%s"}\nhorizon-world={path="%s"}\nhorizon-client-core={path="%s"}\n' % (host / "crates/horizon-items", host / "crates/horizon-world", host / "crates/horizon-client-core"))
    # Resolve from the host lock and reject any changed transitive identity.
    host_lock = (host / "Cargo.lock").read_text()
    (path / "Cargo.lock").write_text(host_lock + '\n[[package]]\nname="drain-container-fixture"\nversion="0.1.0"\ndependencies=["horizon-items", "horizon-world", "horizon-client-core"]\n')
    subprocess.run(["cargo", "metadata", "--offline", "--manifest-path", str(path / "Cargo.toml"), "--format-version", "1"], check=True, stdout=subprocess.DEVNULL)
    def identity(package):
        return tuple(package.get(key) for key in ("name", "version", "source", "checksum"))
    allowed = {identity(package) for package in tomllib.loads(host_lock)["package"]}
    unknown = [identity(package) for package in tomllib.loads((path / "Cargo.lock").read_text())["package"]
               if package["name"] != "drain-container-fixture" and identity(package) not in allowed]
    if unknown:
        raise SystemExit(f"Fixture dependencies differ from pinned host lock: {unknown}")
    result = subprocess.run(["cargo", "test", "--locked", "--offline", "--manifest-path", str(path / "Cargo.toml"), "--", "--nocapture"], check=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
    print(result.stdout)
    for case in ("full_tank_and_empty_tank_consume_container_action_without_mutation", "noncontainers_and_unformed_drains_still_pass", "successful_and_rejected_save_paths_preserve_existing_item_transaction", "drain_held_preserves_raw_controller_and_item_fields", "drain_held_refuses_noncanonical_tanks_and_retained_wire", "drain_held_preservation_save_failure_and_creative_paths", "drain_held_preserves_native_tank_and_row_extensions", "drain_held_opaque_row_removal_and_failed_save_leave_items_unchanged", "tank_only_mode_processing_keeps_machine_inventory_separate", "controller_carried_local_preserves_raw_fields_and_counts", "controller_carried_local_selects_exact_potion_layer", "controller_carried_refusals_leave_cursor_and_save_unchanged", "direct_melter_and_fuel_exchanges_preserve_raw_metadata", "direct_owned_fuel_exchange_preserves_controller_and_extensions", "direct_tank_exchange_failures_do_not_publish_slots", "fluid_slot_pickup_and_quick_input_preserve_machine_metadata", "fluid_slot_output_pickup_and_reprocessing_keep_row_extensions", "held_fuel_all_eight_tanks_preserve_data_and_inventory", "held_fuel_aliases_wire_and_failed_save_never_publish", "held_fuel_callback_changes_preserve_newer_state", "melter_held_use_preserves_data_and_publishes_after_save", "melter_held_use_rejects_changed_preparation_without_menu_fallback", "melter_held_use_aliases_and_opaque_hands_only_open_menu", "tconstruct_source_smeltery_fuel::heater_tests::heater_debit_preserves_fuel_components_and_original_stack", "tconstruct_source_smeltery_fuel::heater_tests::heater_remainder_routing_matches_source_slot_rules", "tconstruct_source_smeltery_fuel::heater_tests::heater_debit_rejects_zero_and_retained_wire_without_a_plan", "portable_tank_drop_keeps_raw_payload_for_all_twelve_items", "portable_tank_payload_refuses_lossy_or_invalid_owned_fields", "portable_tank_drop_only_consumes_matching_marker", "portable_recipe_fill_preserves_all_twelve_tanks", "portable_recipe_fill_rejects_ambiguous_and_opaque_inputs", "portable_recipe_fill_requires_exact_current_tank", "portable_recipe_fill_keeps_extensions_through_casting_completion"):
        if f"test {case} ... ok" not in result.stdout.splitlines():
            raise SystemExit(f"Required route test did not execute: {case}")
