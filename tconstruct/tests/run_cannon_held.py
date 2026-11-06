#!/usr/bin/env python3
"""Run production Cannon interaction and native inventory planner with real Horizon state types and mocked world.

HORIZON_HOST must name the pinned host checkout. Cargo dependencies are real;
World storage, block/loading state and inventory callbacks are fixtures.
"""
from fixture_pluto_dependency import add_inventory_api, inventory_api_identity
import os, pathlib, subprocess, tempfile, tomllib
root = pathlib.Path(__file__).resolve().parents[1]
host = pathlib.Path(os.environ["HORIZON_HOST"]).resolve()
expected = "624c4f6265c02296fbe8a80f029dd0d987fb94c0"
actual = subprocess.check_output(["git", "-C", str(host), "rev-parse", "HEAD"], text=True).strip()
dirty = subprocess.check_output(["git", "-C", str(host), "status", "--porcelain", "--untracked-files=no"], text=True)
if actual != expected or dirty:
    raise SystemExit(f"HORIZON_HOST must have a clean tracked tree at {expected}")
source = (root / "mixins/tconstruct/source-machine-runtime.rs").read_text()
loader = source[source.index("pub(super) fn cannon_at("):source.index("pub(super) fn save_cannon(")]
save = source[source.index("pub(super) fn save_cannon("):source.index("pub(super) fn interact_cannon(")]
casting = source[source.index("pub(super) fn casting_tank_at("):source.index("\nfn notify_casting_comparator(")]
casting_interaction = source[source.index("pub(super) fn process_casting_tank("):source.index("pub(super) fn cannon_at(")]
start = source.index("pub(super) fn interact_cannon(")
end = source.index("\n/// A formed drain", start)
route = source[start:end]
host_game = (host / "crates/horizon-client/src/game.rs").read_text()
inventory_add = host_game[host_game.index("fn inventory_add("):host_game.index("\nfn facing_offset(")]
inventory_add = inventory_add.replace("shield::OFFHAND_SLOT", "40")
hopper_source = (root / "mixins/tconstruct/source-chute-hopper.rs").read_text()
hopper = hopper_source[hopper_source.index("fn cannon_transfer("):hopper_source.index("// The source sided inventory")]
hopper = hopper.replace("fn cannon_transfer(", "pub(super) fn cannon_transfer(", 1)
hopper = hopper.replace("crate::game::Game", "crate::Game").replace("crate::game::tconstruct_source_machine_runtime", "crate::runtime").replace("crate::game::tconstruct_source_melter_store", "crate::store")
hopper = hopper.replace("use horizon_client::{containers::BlockEntity, pluto_inventory::{plan_between, StackSnapshot}};", "use crate::{BlockEntity, pluto_inventory::{plan_between, StackSnapshot}};")
template = (root / "tests/rust/cannon_held_fixture.rs").read_text()
template = template.replace("__MIXINS__", str(root / "mixins/tconstruct")).replace("__PLUTO__", str(root.parent / "pluto-lib/mixins/pluto_native_creative/stack-fluid.rs"))
template = template.replace("// __CANNON_HOPPER__", hopper)
template = template.replace("// __INVENTORY_ADD__", inventory_add)
template = template.replace("// __CANNON_AT__", loader).replace("// __CANNON_HELD__", route).replace("// __CANNON_SAVE__", save).replace("// __CASTING_TANK_SAVE__", casting).replace("// __CASTING_TANK_INTERACTION__", casting_interaction)
with tempfile.TemporaryDirectory(prefix="cannon-held-") as tmp:
    path = pathlib.Path(tmp)
    (path / "src").mkdir()
    (path / "src/lib.rs").write_text(template)
    (path / "Cargo.toml").write_text('[package]\nname="cannon-held-fixture"\nversion="0.1.0"\nedition="2024"\n[dependencies]\nhorizon-items={path="%s"}\nhorizon-world={path="%s"}\nhorizon-client-core={path="%s"}\n' % (host / "crates/horizon-items", host / "crates/horizon-world", host / "crates/horizon-client-core"))
    # Resolve from the host lock and reject any changed transitive identity.
    add_inventory_api(path / "Cargo.toml", root.parent)
    host_lock = (host / "Cargo.lock").read_text()
    (path / "Cargo.lock").write_text(host_lock + '\n[[package]]\nname="cannon-held-fixture"\nversion="0.1.0"\ndependencies=["horizon-items", "horizon-world", "horizon-client-core"]\n')
    subprocess.run(["cargo", "metadata", "--offline", "--manifest-path", str(path / "Cargo.toml"), "--format-version", "1"], check=True, stdout=subprocess.DEVNULL)
    def identity(package):
        return tuple(package.get(key) for key in ("name", "version", "source", "checksum"))
    allowed = {identity(package) for package in tomllib.loads(host_lock)["package"]}
    allowed.add(inventory_api_identity(root.parent))
    unknown = [identity(package) for package in tomllib.loads((path / "Cargo.lock").read_text())["package"]
               if package["name"] != "cannon-held-fixture" and identity(package) not in allowed]
    if unknown:
        raise SystemExit(f"Fixture dependencies differ from pinned host lock: {unknown}")
    result = subprocess.run(["cargo", "test", "--locked", "--offline", "--manifest-path", str(path / "Cargo.toml"), "--", "--nocapture"], stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
    print(result.stdout)
    result.check_returncode()
    for case in ('machine_save_adapters_preserve_native_codec_eligibility', 'cannon_hopper_preserves_unselected_slot_wire_changes', 'cannon_held_fill_drain_preserve_raw_and_live_items', 'cannon_held_snapshot_changes_never_commit_or_notify', 'cannon_held_unsafe_metadata_and_failed_save_preserve_inventory', 'cannon_item_saves_preserve_extensions_before_publishing_slots', 'cannon_item_saves_refuse_opaque_changes_and_stale_live_state', 'cannon_item_saves_refuse_retained_wire_and_failed_persistence', 'cannon_hopper_push_pull_preserve_data_and_counts', 'cannon_hopper_callback_changes_preserve_both_inventories', 'cannon_hopper_failed_save_rolls_back_and_invalid_endpoints_wait', 'cannon_item_click_insert_and_empty_hand_extract_preserve_data', 'cannon_item_click_uses_native_add_order_and_partial_transfer', 'cannon_item_click_no_space_and_unsafe_stacks_do_not_save', 'cannon_item_click_failed_save_preserves_player_inventory', 'casting_tank_save_preserves_extensions_and_processed_containers', 'casting_tank_save_refuses_opaque_slot_changes_and_stale_state', 'casting_tank_process_and_save_refuse_wire_and_failed_commit', 'casting_tank_held_transfers_preserve_metadata_and_counts', 'casting_tank_held_callbacks_never_overwrite_changed_state', 'casting_tank_upper_insert_take_and_callback_guard', 'casting_tank_interaction_refuses_wire_and_failed_save'):
        if f"test {case} ... ok" not in result.stdout.splitlines():
            raise SystemExit(f"Required route test did not execute: {case}")
