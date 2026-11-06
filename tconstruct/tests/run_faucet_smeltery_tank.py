#!/usr/bin/env python3
"""Test production route with real host state/NBT; world and structure are mocks."""
from fixture_pluto_dependency import add_inventory_api, lock_fixture_to_host, run_locked_fixture
import os, pathlib, tempfile
root=pathlib.Path(__file__).resolve().parents[1]; host=pathlib.Path(os.environ['HORIZON_HOST']).resolve()
s=(root/'mixins/tconstruct/source-machine-runtime.rs').read_text()
helpers=s[s.index('fn selected_smeltery_fluid('):s.index('fn matched_casting(')]
route=s[s.index('fn patch_smeltery_controller_tank('):s.index('fn tank_casting_recipe_current(')] + s[s.index('fn faucet_network_tag_safe('):s.index('fn tank_to_casting_tank(')]

template=(root/'tests/rust/faucet_smeltery_tank_fixture.rs').read_text().replace('__MIXINS__',str(root/'mixins/tconstruct')).replace('__PLUTO__',str(root.parent/'pluto-lib/mixins/pluto_native_creative/stack-fluid.rs')).replace('// __SMELTERY_HELPERS__',helpers).replace('// __PRODUCTION_ROUTE__',route)
with tempfile.TemporaryDirectory(prefix='smeltery-faucet-') as tmp:
 p=pathlib.Path(tmp);(p/'src').mkdir();(p/'src/lib.rs').write_text(template)
 (p/'Cargo.toml').write_text('[package]\nname="smeltery-faucet-fixture"\nversion="0.1.0"\nedition="2024"\n[dependencies]\nhorizon-items={path="%s"}\nhorizon-world={path="%s"}\nhorizon-client-core={path="%s"}\n' % (host/'crates/horizon-items',host/'crates/horizon-world',host/'crates/horizon-client-core'))
 add_inventory_api(p / 'Cargo.toml', root.parent)
 lock_fixture_to_host(p / 'Cargo.toml', host, root.parent)
 run_locked_fixture(p / 'Cargo.toml', (
     'runtime::reserves_atomically_and_preserves_opaque_controller',
     'runtime::tagged_fluid_and_unloaded_controller_wait',
     'runtime::potion_layers_conserve_identity_and_rollback',
     'runtime::potion_custom_effects_pass_only_nonnetwork',
     'runtime::potion_network_complex_and_invalid_refuse',
     'runtime::invalid_pending_or_output_tag_waits',
     'runtime::stale_controller_unknown_tank_and_duct_fail_closed',
 ))
