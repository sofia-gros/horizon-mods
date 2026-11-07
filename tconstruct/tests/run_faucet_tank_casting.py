#!/usr/bin/env python3
"""Run production faucet route with real Horizon state types and mocked world.

HORIZON_HOST must name the pinned host checkout. Cargo dependencies are real;
recipe selection, world storage, chunk state and networking are fixtures.
"""
from fixture_pluto_dependency import add_inventory_api, lock_fixture_to_host, run_locked_fixture
import os, pathlib, tempfile
root = pathlib.Path(__file__).resolve().parents[1]
host = pathlib.Path(os.environ["HORIZON_HOST"]).resolve()
source = (root / "mixins/tconstruct/source-machine-runtime.rs").read_text()
route = (source[source.index("fn tank_casting_recipe_current("):source.index("fn tank_to_cannon(")]
         + source[source.index("enum CastingInput {"):source.index("pub(super) fn activate_faucet(")])
template = (root / "tests/rust/faucet_tank_casting_fixture.rs").read_text()
template = template.replace("__MIXINS__", str(root / "mixins/tconstruct")).replace("__PLUTO__",str(root.parent / "pluto-lib/mixins/pluto_native_creative/stack-fluid.rs")).replace("// __PRODUCTION_ROUTE__", route)
with tempfile.TemporaryDirectory(prefix="faucet-casting-") as tmp:
    path = pathlib.Path(tmp)
    (path / "src").mkdir()
    (path / "src/lib.rs").write_text(template)
    (path / "Cargo.toml").write_text('[package]\nname="faucet-casting-fixture"\nversion="0.1.0"\nedition="2024"\n[dependencies]\nhorizon-items={path="%s"}\nhorizon-world={path="%s"}\nhorizon-client-core={path="%s"}\n' % (host / "crates/horizon-items", host / "crates/horizon-world", host / "crates/horizon-client-core"))
    add_inventory_api(path / "Cargo.toml", root.parent)
    lock_fixture_to_host(path / 'Cargo.toml', host, root.parent)
    run_locked_fixture(path / 'Cargo.toml', (
        'tconstruct_source_machine_runtime::reserve_tick_and_failed_write',
        'tconstruct_source_machine_runtime::stale_recipe_and_unknown_casting_refuse',
        'tconstruct_source_machine_runtime::portable_source_preserves_slots_and_refuses_bad_state',
        'tconstruct_source_machine_runtime::source_container_reprocess_is_atomic',
        'tconstruct_source_machine_runtime::source_refill_tag_waits_only_after_untagged_packet_finishes',
        'tconstruct_source_machine_runtime::recheck_target_and_recipe_after_source_process',
        'tconstruct_source_machine_runtime::portable_source_tagged_and_unknown_refuse',
        'tconstruct_source_machine_runtime::unknown_source_and_unloaded_refuse',
    ))
