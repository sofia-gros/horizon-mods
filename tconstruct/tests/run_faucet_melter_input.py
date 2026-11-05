#!/usr/bin/env python3
"""Run the production Melter input route with real state/NBT and a mock world."""
import os, pathlib, subprocess, tempfile
root = pathlib.Path(__file__).resolve().parents[1]
host = pathlib.Path(os.environ['HORIZON_HOST']).resolve()
source = (root / 'mixins/tconstruct/source-machine-runtime.rs').read_text()
kind = source[source.index('pub(super) fn kind('):source.index('fn initial(')]
safety = source[source.index('fn faucet_network_tag_safe('):source.index('fn faucet_supported_potion_tag(')]
helpers = source[source.index('pub(super) fn casting_tank_at('):source.index('pub(super) fn save_casting_tank(')]
helpers += source[source.index('fn selected_smeltery_fluid('):source.index('fn matched_casting(')]
helpers += source[source.index('fn patch_smeltery_controller_tank('):source.index('fn smeltery_to_tank(')]
route = source[source.index('fn source_to_melter_or_lantern('):]
template = (root / 'tests/rust/faucet_melter_input_fixture.rs').read_text()
template = template.replace('__MIXINS__', str(root / 'mixins/tconstruct'))
template = template.replace('__PLUTO__', str(root.parent / 'pluto-lib/mixins/pluto_native_creative/stack-fluid.rs'))
template = template.replace('// __PRODUCTION_ROUTE__', kind + safety + helpers + route)
with tempfile.TemporaryDirectory(prefix='faucet-melter-input-') as tmp:
    p = pathlib.Path(tmp)
    (p / 'src').mkdir()
    (p / 'src/lib.rs').write_text(template)
    (p / 'Cargo.toml').write_text('[package]\nname="faucet-melter-input-fixture"\nversion="0.1.0"\nedition="2024"\n[dependencies]\nhorizon-items={path="%s"}\nhorizon-world={path="%s"}\nhorizon-client-core={path="%s"}\n' % (host / 'crates/horizon-items', host / 'crates/horizon-world', host / 'crates/horizon-client-core'))
    subprocess.run(['cargo', 'generate-lockfile', '--offline', '--manifest-path', str(p / 'Cargo.toml')], check=True)
    subprocess.run(['cargo', 'test', '--locked', '--offline', '--manifest-path', str(p / 'Cargo.toml')], check=True)
