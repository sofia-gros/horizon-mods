#!/usr/bin/env python3
"""Test production transfer route; host state and container transfer are real, world/store mocked."""
from fixture_pluto_dependency import add_inventory_api, lock_fixture_to_host, run_locked_fixture
import os,pathlib,tempfile
root=pathlib.Path(__file__).resolve().parents[1];host=pathlib.Path(os.environ['HORIZON_HOST']).resolve()
s=(root/'mixins/tconstruct/source-machine-runtime.rs').read_text();route=s[s.index('fn faucet_network_tag_safe('):s.index('enum CastingInput {')]
t=(root/'tests/rust/faucet_tagged_local_fixture.rs').read_text().replace('__MIXINS__',str(root/'mixins/tconstruct')).replace('__PLUTO__',str(root.parent/'pluto-lib/mixins/pluto_native_creative/stack-fluid.rs')).replace('// __PRODUCTION_ROUTE__',route)
with tempfile.TemporaryDirectory(prefix='faucet-portable-') as tmp:
 p=pathlib.Path(tmp);(p/'src').mkdir();(p/'src/lib.rs').write_text(t);(p/'Cargo.toml').write_text('[package]\nname="faucet-tagged-fixture"\nversion="0.1.0"\nedition="2024"\n[dependencies]\nhorizon-items={path="%s"}\nhorizon-world={path="%s"}\nhorizon-client-core={path="%s"}\n'%(host/'crates/horizon-items',host/'crates/horizon-world',host/'crates/horizon-client-core'))
 add_inventory_api(p / 'Cargo.toml', root.parent)
 lock_fixture_to_host(p / 'Cargo.toml', host, root.parent)
 run_locked_fixture(p / 'Cargo.toml', (
     'tconstruct_source_machine_runtime::test_tagged_local_potion_fills_bottle',
     'tconstruct_source_machine_runtime::test_tagged_network_refuses_without_mutation',
     'tconstruct_source_machine_runtime::test_tagged_invalid_utf16_and_unknown_effect_refuse',
     'tconstruct_source_machine_runtime::test_tagged_mismatch_and_untagged_network',
     'tconstruct_source_machine_runtime::test_tagged_bare_id_network_refuses',
     'tconstruct_source_machine_runtime::test_tagged_basic_potion_network_allowed',
     'tconstruct_source_machine_runtime::test_tagged_unknown_potion_refuses',
 ))
