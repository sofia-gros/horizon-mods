from pathlib import Path
import unittest
ROOT=Path(__file__).resolve().parents[1]
class CastingPreservation(unittest.TestCase):
    def test_host_writes_merge_but_snapshots_replace(self):
        store=(ROOT/'mixins/tconstruct/source-melter-store.rs').read_text()
        self.assertIn('preserve_smeltery && matches!(key, "tconstruct:table" | "tconstruct:basin")',store)
        self.assertIn('state.to_nbt_preserving(previous)',store)
        self.assertIn('set_metadata_inner(&mut saved.block_entity_sources, pos, key, value, false)',store)
        self.assertIn('set_metadata(sources, *pos, key, value.clone())',store)
    def test_live_items_checked_before_serialization_and_consumption(self):
        runtime=(ROOT/'mixins/tconstruct/source-machine-runtime.rs').read_text()
        body=runtime.split('pub(super) fn interact_casting(',1)[1].split('\nfn faucet_neighbors(',1)[0]
        self.assertLess(body.index('casting_item_save_safe(item)'),body.index('ensure(game'))
        self.assertNotIn('casting.to_nbt()',body)
        self.assertNotIn('next.to_nbt()',body)
        self.assertEqual(body.count('store::write_casting('),5)
        state=(ROOT/'mixins/tconstruct/source-melter-state.rs').read_text()
        self.assertIn('horizon_client::pluto_stack_storage::encode_item_lossless(item).is_some()',state)
        self.assertIn('if !self.items_save_safe() { return None; }',state)
    def test_ci_runs_pinned_production_store_fixture(self):
        ci=(ROOT.parent/'.github/workflows/mixins.yml').read_text()
        self.assertIn('tconstruct/tests/run_casting_preservation_rust.sh',ci)
        runner=(ROOT/'tests/run_casting_preservation_rust.sh').read_text()
        self.assertIn('ebcfa2263aa3d8e399aa993b1e05daf42eb4cfa0',runner)
        self.assertIn('smeltery_nbt_store.rs',runner)
        self.assertIn('unchanged_opaque_items_survive_ticks_but_consumption_aborts_batch',runner)
if __name__=='__main__':unittest.main()
