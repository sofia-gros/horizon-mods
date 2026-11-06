"""Check the runtime adapter uses full-water growth and separate source-water logging."""
import unittest
from pathlib import Path
ROOT = Path(__file__).resolve().parents[1]
class GeodeGrowthWater(unittest.TestCase):
    def test_adapter_uses_distinct_source_conditions(self):
        text=(ROOT/'mixins/tconstruct/source-native-hook-bodies.rs').read_text()
        body=text.split('pub(super) fn tconstruct_geode_random_tick',1)[1].split('pub(super) fn ',1)[0]
        self.assertIn('old_name == "minecraft:water", fluid.full',body)
        self.assertIn('fluid.id == "minecraft:water"',body)
        self.assertNotIn('fluid == FluidState::WATER',body)
        self.assertIn('tconstruct:budding_sky_slime_crystal',body)
        self.assertNotIn('"minecraft:budding_amethyst"',body)
if __name__=='__main__': unittest.main()
