"""Ensure slime grass spread reads host's resolved water tag."""
import unittest
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
class SlimeGrassWaterTag(unittest.TestCase):
    def test_spread_uses_water_tag_not_fixed_fluid_kinds(self):
        text=(ROOT/'mixins/tconstruct/source-native-hook-bodies.rs').read_text()
        body=text.split('pub(super) fn tconstruct_slimy_grass_tick',1)[1].split('pub(super) fn ',1)[0]
        self.assertIn('__horizon_mixins_tconstruct_resolved_fluid_tags()',body)
        self.assertIn('resolved_member(',body)
        self.assertIn('local_tag_member(',body)
        self.assertIn('pluto_fluid_facts::native_ids()',body)
        self.assertIn('minecraft:water',body)
        self.assertIn('else { continue; }',body)
        self.assertNotIn('FluidKind::FlowingWater',body)
        self.assertIn('if nether ||',body)
if __name__=='__main__': unittest.main()
