"""Wiring checks for potion names in Copper Gauge action-bar output."""
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1] / 'mixins' / 'tconstruct'

class PotionGaugeNames(unittest.TestCase):
    def test_gauge_uses_tag_for_potion_only(self):
        gauge = (ROOT / 'source-gauge.rs').read_text()
        self.assertIn('if fluid == "tconstruct:potion"', gauge)
        self.assertIn('potion_fluid_name(tank.tag.as_ref())', gauge)
        self.assertIn('pluto_fluid::first(&game.world, at, facing).is_some()', gauge)
        self.assertIn('(!native).then(||', gauge)
        self.assertIn('unwrap_or_else(|| "fluid.tconstruct.potion".into())', gauge)
        self.assertIn('Text::translate(&key, &[])', gauge)

    def test_host_harness_module_is_in_ci(self):
        workflow = (ROOT.parents[2] / '.github' / 'workflows' / 'mixins.yml').read_text()
        self.assertIn('tconstruct_source_smeltery_fuel::tests', workflow)

if __name__ == '__main__':
    unittest.main()
