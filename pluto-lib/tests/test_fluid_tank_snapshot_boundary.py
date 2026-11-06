"""Checks the read-only tank boundary; native Rust tests run in the host batch."""
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[2]


class FluidTankSnapshotBoundary(unittest.TestCase):
    def test_raw_reader_remains_available_and_snapshot_uses_it_once(self):
        source = (ROOT / 'pluto-lib/mixins/pluto_native_creative/fluid-read.rs').read_text()
        self.assertIn('pub fn first(', source)
        self.assertIn('pub fn first_snapshot(', source)
        self.assertIn('first(world, pos, face)\n        .map(TankSnapshot::try_from)\n        .transpose()', source)
        self.assertNotIn('tconstruct', source)
        self.assertNotIn('set_block', source)

    def test_snapshot_preserves_the_existing_validation_and_empty_rules(self):
        source = (ROOT / 'pluto-lib/mixins/pluto_native_creative/fluid-read.rs').read_text()
        for marker in ('raw.capacity <= 0', 'u32::try_from(raw.amount)',
                       'amount > capacity', 'if amount == 0',
                       'raw.fluid.ok_or(InvalidTank::MissingFluid)',
                       'metadata_complete: false'):
            self.assertIn(marker, source)
        self.assertIn('impl std::error::Error for InvalidTank', source)

    def test_gauge_carries_metadata_completeness_instead_of_reprobing(self):
        source = (ROOT / 'tconstruct/mixins/tconstruct/source-gauge.rs').read_text()
        self.assertEqual(source.count('pluto_fluid::first_snapshot('), 1)
        self.assertNotIn('pluto_fluid::first(', source)
        self.assertIn('native.metadata_complete', source)
        self.assertIn('Some((tank, metadata_complete))', source)
        self.assertIn('metadata_complete.then(||', source)
        self.assertNotIn('u32::try_from(native.', source)

    def test_invalid_native_provider_cannot_fall_through_to_local_data(self):
        source = (ROOT / 'tconstruct/mixins/tconstruct/source-gauge.rs').read_text()
        body = source.split('match horizon_client::pluto_fluid::first_snapshot', 1)[1]
        body = body.split('if !world.block_entities.contains_key', 1)[0]
        self.assertIn('Err(_) => return None', body)
        self.assertIn('Ok(None) => {}', body)
        self.assertIn('Ok(Some(native)) => return Some(', body)


if __name__ == '__main__':
    unittest.main()
