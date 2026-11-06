"""Guard the provisional, water-only Hydraulic path and its player warning."""
from pathlib import Path
import json
import unittest
import zipfile

ROOT = Path(__file__).resolve().parents[1]

class HydraulicWater(unittest.TestCase):
    def test_pinned_recipe_and_formula(self):
        with zipfile.ZipFile(ROOT / 'content/reference/tconstruct-modifiers.zip') as z:
            modifier = json.loads(z.read('data/tconstruct/tinkering/modifiers/hydraulic.json'))
        module = modifier['modules'][0]
        self.assertEqual(module['type'], 'tconstruct:conditional_mining_speed')
        self.assertTrue(module['require_effective'])
        self.assertEqual(module['variables']['bonus']['if_true']['if_false'], 40.0)
        self.assertEqual(module['variables']['bonus']['if_true']['if_true'], 8.0)
        with zipfile.ZipFile(ROOT / 'content/reference/tconstruct-recipes.zip') as z:
            recipe = json.loads(z.read('data/tconstruct/recipes/tools/modifiers/upgrade/hydraulic_from_shard.json'))
        self.assertEqual(recipe['needed_per_level'], 36)
        self.assertEqual(recipe['tools']['tag'], 'tconstruct:modifiable/harvest')
    def test_warning_and_actor_water_sample_are_wired(self):
        context = (ROOT / 'mixins/tconstruct/source-mining-context-shared.rs').read_text()
        self.assertIn('facts.previous_eye_in_water.then_some(aqua_affinity)', context)
        self.assertIn('conditional_with_water(', context)
        tooltip = (ROOT / 'mixins/tconstruct/part-name-injection.rs').read_text()
        station = (ROOT / 'mixins/tconstruct/station-info-ui.rs').read_text()
        for text in (tooltip, station):
            self.assertIn('Hydraulic: underwater only; rain bonus unavailable', text)
        # The upgrade ID is a String; as_ref() is ambiguous in this equality.
        self.assertIn('if id.as_str() == "tconstruct:hydraulic"', station)
        self.assertIn('Hydraulic: water only', station)
        self.assertIn('Rain bonus unavailable', station)
        self.assertIn('("hydraulic", 36)', tooltip)
        self.assertIn('"tconstruct:hydraulic" | "tconstruct:sharpness" => 36', station)

if __name__ == '__main__':
    unittest.main()
