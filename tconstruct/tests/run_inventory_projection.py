#!/usr/bin/env python3
"""Check real chest/Smeltery projections against the clean pinned official host."""
from fixture_pluto_dependency import add_inventory_api, lock_fixture_to_host
import json
import os
from pathlib import Path
import subprocess
import tempfile

root = Path(__file__).resolve().parents[2]
host = Path(os.environ["HORIZON_HOST"]).resolve()
with tempfile.TemporaryDirectory(prefix="inventory-projection-") as temp:
    work = Path(temp)
    def quoted(path):
        return json.dumps(str(path))
    (work / "Cargo.toml").write_text(f'''[package]
name = "inventory-projection-fixture"
version = "0.1.0"
edition = "2024"
[dependencies]
horizon-items = {{ path = {quoted(host / "crates/horizon-items")} }}
horizon-world = {{ path = {quoted(host / "crates/horizon-world")} }}
[lib]
path = "lib.rs"
''')
    add_inventory_api(work / "Cargo.toml", root)
    (work / "lib.rs").write_text(f'''
include!({quoted(root / "tconstruct/tests/rust/pluto_storage_fixture.rs")});
#[path = {quoted(root / "tconstruct/mixins/tconstruct/chest-inventory-api-game.rs")}]
mod game_inventory_api;
#[path = {quoted(root / "tconstruct/tests/source_tinker_chests.rs")}]
mod chests;
#[path = {quoted(root / "tconstruct/tests/rust/smeltery_pickup_single.rs")}]
mod smeltery;
''')
    lock_fixture_to_host(work / "Cargo.toml", host, root)
    result = subprocess.run(["cargo", "test", "--locked", "--offline", "--manifest-path", str(work / "Cargo.toml")],
                            text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    print(result.stdout)
    result.check_returncode()
    for case in (
        "chests::projection_regressions::both_chest_views_match_original_mapping_with_opaque_items",
        "chests::projection_regressions::projected_stack_edits_do_not_change_source_cells",
        "chests::rules::tests::guest_projection_has_only_real_page_slots",
        "chests::rules::tests::builder_side_projection_never_exposes_transport_padding",
        "smeltery::tests::shared_projection_keeps_reserved_slots_layout_and_wire",
        "smeltery::tests::nested_opaque_held_items_never_reach_single_insert_encoding",
        "smeltery::tests::nested_opaque_sent_or_expected_pages_fail_closed",
        "smeltery::tconstruct_source_smeltery_page::tests::projection_preserves_each_viewers_container_slots",
        "smeltery::tconstruct_source_smeltery_page::tests::rejects_ghost_and_stale_pages_without_changing_real_slots",
    ):
        if f"test {case} ... ok" not in result.stdout.splitlines():
            raise SystemExit(f"Required projection test did not pass: {case}")
