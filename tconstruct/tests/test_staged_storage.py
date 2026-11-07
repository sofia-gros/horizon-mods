"""Proposal staging is reusable; authoritative storage and schemas are not."""
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[2]


class StagedStorage(unittest.TestCase):
    def test_pure_api_has_no_consumer_or_native_storage_dependencies(self):
        source = (ROOT / 'pluto-lib/mixins/pluto_native_creative/inventory-api/src/storage.rs').read_text()
        for dependency in ['horizon_', 'tconstruct:', 'block_entity_sources', 'world.modified']:
            self.assertNotIn(dependency, source)
        self.assertIn('load_copy(entry.key())?', source)
        self.assertIn('if !apply_to_copy(proposal, change) { return None; }', source)

    def test_store_keeps_schema_and_authoritative_publication(self):
        source = (ROOT / 'tconstruct/mixins/tconstruct/source-melter-store.rs').read_text()
        body = source.split('pub(super) fn write_many(', 1)[1]
        stage = body.index('pluto_inventory::storage::stage_updates(')
        publish = body.index('for (chunk_pos, sources) in staged')
        self.assertLess(stage, publish)
        self.assertIn('set_metadata(sources, *pos, key, value.clone())', body[:publish])
        self.assertIn('block_entity_sources.clone()', body[:publish])
        self.assertNotIn('world.chunks.get_mut', body[:publish])
        self.assertIn('saved.block_entity_sources = sources', body[publish:])
        self.assertIn('world.modified.insert(chunk_pos)', body[publish:])
        self.assertIn('"tconstruct:smeltery"', source.split('pub(super) fn write_many(', 1)[0])

    def test_store_fixture_requires_missing_and_repeated_partition_cases(self):
        runner = (ROOT / 'tconstruct/tests/run_casting_preservation_rust.sh').read_text()
        self.assertIn('write_many_preserves_repeated_update_order_across_partitions', runner)
        self.assertIn('write_many_missing_partition_and_empty_changes_never_publish', runner)


if __name__ == '__main__':
    unittest.main()
