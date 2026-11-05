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
patch = source[source.index("fn patch_smeltery_controller_tank("):source.index("\nfn validated_drain(")]
direct_source = (root / "mixins/tconstruct/source-fluid-direct.rs").read_text()
direct = direct_source[direct_source.index("pub(super) fn local("):direct_source.index("pub(super) fn guest(")]
melter_exchange = direct_source[direct_source.index("fn melter_exchange("):direct_source.index("pub(super) fn melter_local(")].replace("fn melter_exchange(", "pub(super) fn melter_exchange(", 1)
fuel_exchange = direct_source[direct_source.index("fn fuel_exchange("):direct_source.index("pub(super) fn fuel_local(")].replace("fn fuel_exchange(", "pub(super) fn fuel_exchange(", 1)
kind = source[source.index("pub(super) fn kind("):source.index("\nfn initial(")]
template = (root / "tests/rust/drain_container_fixture.rs").read_text().replace("// __PRODUCTION_DIRECT_LOCAL__", direct).replace("// __PRODUCTION_MELTER_EXCHANGE__", melter_exchange).replace("// __PRODUCTION_FUEL_EXCHANGE__", fuel_exchange).replace("// __PRODUCTION_KIND__", kind).replace("// __PRODUCTION_PATCH__", patch)
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
    for case in ("full_tank_and_empty_tank_consume_container_action_without_mutation", "noncontainers_and_unformed_drains_still_pass", "successful_and_rejected_save_paths_preserve_existing_item_transaction", "drain_held_preserves_raw_controller_and_item_fields", "drain_held_refuses_noncanonical_tanks_and_retained_wire", "drain_held_preservation_save_failure_and_creative_paths", "drain_held_preserves_native_tank_and_row_extensions", "drain_held_opaque_row_removal_and_failed_save_leave_items_unchanged", "tank_only_mode_processing_keeps_machine_inventory_separate", "controller_carried_local_preserves_raw_fields_and_counts", "controller_carried_local_selects_exact_potion_layer", "controller_carried_refusals_leave_cursor_and_save_unchanged", "direct_melter_and_fuel_exchanges_preserve_raw_metadata", "direct_owned_fuel_exchange_preserves_controller_and_extensions", "direct_tank_exchange_failures_do_not_publish_slots", "fluid_slot_pickup_and_quick_input_preserve_machine_metadata", "fluid_slot_output_pickup_and_reprocessing_keep_row_extensions"):
        if f"test {case} ... ok" not in result.stdout.splitlines():
            raise SystemExit(f"Required route test did not execute: {case}")
