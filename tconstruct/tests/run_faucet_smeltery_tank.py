#!/usr/bin/env python3
"""Test production route with real host state/NBT; world and structure are mocks."""
import os, pathlib, subprocess, tempfile
root=pathlib.Path(__file__).resolve().parents[1]; host=pathlib.Path(os.environ['HORIZON_HOST']).resolve()
s=(root/'mixins/tconstruct/source-machine-runtime.rs').read_text()
helpers=s[s.index('fn selected_smeltery_fluid('):s.index('fn matched_casting(')]
route=s[s.index('pub(super) fn cannon_at('):s.index('pub(super) fn save_cannon(')] + s[s.index('pub(super) fn casting_at('):s.index('// Empty-cast items stand in')]+s[s.index('fn prepare_faucet_casting('):s.index('fn tick_smeltery_faucet(')]+s[s.index('fn tank_casting_recipe_current('):s.index('// Bounded built-in tank -> casting route.')]+s[s.index('fn patch_smeltery_controller_tank('):s.index('fn tank_casting_recipe_current(')] + s[s.index('fn faucet_network_tag_safe('):s.index('fn tank_to_casting_tank(')]+s[s.index('fn machine_to_casting('):]

tank_source=(root/'mixins/tconstruct/source-faucet-tank.rs').read_text()
melter_patch=tank_source[tank_source.index('pub(super) fn patch_melter_tank('):tank_source.index('fn patch_faucet(')]
template=(root/'tests/rust/faucet_smeltery_tank_fixture.rs').read_text().replace('__MIXINS__',str(root/'mixins/tconstruct')).replace('__PLUTO__',str(root.parent/'pluto-lib/mixins/pluto_native_creative/stack-fluid.rs')).replace('// __SMELTERY_HELPERS__',helpers).replace('// __PRODUCTION_ROUTE__',route).replace('// __MELTER_PATCH__',melter_patch)
with tempfile.TemporaryDirectory(prefix='smeltery-faucet-') as tmp:
 p=pathlib.Path(tmp);(p/'src').mkdir();(p/'src/lib.rs').write_text(template)
 (p/'Cargo.toml').write_text('[package]\nname="smeltery-faucet-fixture"\nversion="0.1.0"\nedition="2024"\n[dependencies]\nhorizon-items={path="%s"}\nhorizon-world={path="%s"}\nhorizon-client-core={path="%s"}\n' % (host/'crates/horizon-items',host/'crates/horizon-world',host/'crates/horizon-client-core'))
 subprocess.run(['cargo','generate-lockfile','--offline','--manifest-path',str(p/'Cargo.toml')],check=True)
 subprocess.run(['cargo','test','--locked','--offline','--manifest-path',str(p/'Cargo.toml')],check=True)
