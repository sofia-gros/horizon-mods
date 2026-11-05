from pathlib import Path
import unittest
ROOT=Path(__file__).resolve().parents[1]
class CastingPreservation(unittest.TestCase):
    def test_host_writes_merge_but_snapshots_replace(self):
        store=(ROOT/'mixins/tconstruct/source-melter-store.rs').read_text()
        self.assertIn('preserve_smeltery && matches!(key, "tconstruct:table" | "tconstruct:basin")',store)
        self.assertIn('state.to_nbt_preserving(previous)',store)
        self.assertIn('set_metadata_inner(&mut saved.block_entity_sources, pos, key, value, false)',store)
        self.assertIn('set_metadata(staged.get_mut(&chunk_pos).unwrap()',store)
    def test_ci_runs_pinned_production_store_fixture(self):
        ci=(ROOT.parent/'.github/workflows/mixins.yml').read_text()
        self.assertIn('tconstruct/tests/run_casting_preservation_rust.sh',ci)
        runner=(ROOT/'tests/run_casting_preservation_rust.sh').read_text()
        self.assertIn('0c7b5b61a5e746d621afffba919b12aeccf9c0fa',runner)
        self.assertIn('smeltery_nbt_store.rs',runner)
        self.assertIn('unchanged_opaque_items_survive_ticks_but_consumption_aborts_batch',runner)
if __name__=='__main__':unittest.main()
