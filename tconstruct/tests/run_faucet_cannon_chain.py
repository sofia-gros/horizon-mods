#!/usr/bin/env python3
"""Test production Cannon chain route with real state and mocked world/store."""
import os,pathlib,subprocess,tempfile
root=pathlib.Path(__file__).resolve().parents[1];host=pathlib.Path(os.environ['HORIZON_HOST']).resolve()
s=(root/'mixins/tconstruct/source-machine-runtime.rs').read_text();route=s[s.index('pub(super) fn cannon_at('):s.index('pub(super) fn save_cannon(')] + s[s.index('fn faucet_network_tag_safe('):s.index('fn faucet_supported_potion_tag(')] + s[s.index('fn cannon_to_cannon('):]
t=(root/'tests/rust/faucet_cannon_chain_fixture.rs').read_text().replace('__MIXINS__',str(root/'mixins/tconstruct')).replace('__PLUTO__',str(root.parent/'pluto-lib/mixins/pluto_native_creative/stack-fluid.rs')).replace('// __PRODUCTION_ROUTE__',route)
with tempfile.TemporaryDirectory(prefix='faucet-portable-') as tmp:
 p=pathlib.Path(tmp);(p/'src').mkdir();(p/'src/lib.rs').write_text(t);(p/'Cargo.toml').write_text('[package]\nname="faucet-cannon-chain-fixture"\nversion="0.1.0"\nedition="2024"\n[dependencies]\nhorizon-items={path="%s"}\nhorizon-world={path="%s"}\nhorizon-client-core={path="%s"}\n'%(host/'crates/horizon-items',host/'crates/horizon-world',host/'crates/horizon-client-core'))
 subprocess.run(['cargo','generate-lockfile','--offline','--manifest-path',str(p/'Cargo.toml')],check=True);subprocess.run(['cargo','test','--locked','--offline','--manifest-path',str(p/'Cargo.toml')],check=True)
