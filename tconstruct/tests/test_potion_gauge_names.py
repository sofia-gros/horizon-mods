"""Wiring checks for potion names in Copper Gauge action-bar output."""
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1] / 'mixins' / 'tconstruct'

class PotionGaugeNames(unittest.TestCase):
    def test_gauge_uses_tag_for_potion_only(self):
        gauge = (ROOT / 'source-gauge.rs').read_text()
        self.assertIn('if fluid == "tconstruct:potion"', gauge)
        self.assertIn('potion_fluid_name(tank.tag.as_ref())', gauge)
        self.assertIn('pluto_fluid::first_snapshot(world, pos, face)', gauge)
        self.assertIn('native.metadata_complete', gauge)
        self.assertNotIn('pluto_fluid::first(&game.world, at, facing)', gauge)
        self.assertIn('metadata_complete.then(||', gauge)
        self.assertIn('unwrap_or_else(|| "fluid.tconstruct.potion".into())', gauge)
        self.assertIn('Text::translate(&key, &[])', gauge)

    def test_milk_uses_the_shared_display_name_with_fallback(self):
        gauge = (ROOT / 'source-gauge.rs').read_text()
        self.assertIn('display_fluid == "minecraft:milk"', gauge)
        self.assertIn('tconstruct_source_melter_render::fluid_name(game, display_fluid)', gauge)
        render = (ROOT / 'source-melter-render.rs').read_text()
        self.assertIn('fluid_type.minecraft.milk', render)
        self.assertIn('unwrap_or_else(|| "Milk".to_owned())', render)

    def test_host_harness_module_is_in_ci(self):
        github = ROOT.parents[2] / '.github'
        workflow = (github / 'workflows' / 'mixins.yml').read_text() + (github / 'parked' / 'mixins-full-checks.yml').read_text()
        self.assertIn('tconstruct_source_smeltery_fuel::tests', workflow)

if __name__ == '__main__':
    unittest.main()
