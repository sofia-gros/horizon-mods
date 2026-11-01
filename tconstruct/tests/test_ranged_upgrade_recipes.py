import json
from pathlib import Path
import unittest
import zipfile

ROOT = Path(__file__).resolve().parents[1]

class RangedUpgradeRecipes(unittest.TestCase):
    def test_pinned_incremental_inputs_and_limits(self):
        with zipfile.ZipFile(ROOT / 'content/reference/tconstruct-recipes.zip') as archive:
            for name, item, needed, maximum in [('quick_charge', 'minecraft:magma_cream', 5, 4), ('trueshot', 'minecraft:target', 10, 3)]:
                recipe = json.loads(archive.read(f'data/tconstruct/recipes/tools/modifiers/upgrade/{name}.json'))
                self.assertEqual(recipe['type'], 'tconstruct:incremental_modifier')
                self.assertEqual(recipe['input'], {'item': item})
                self.assertEqual((recipe['amount_per_item'], recipe['needed_per_level'], recipe['level']['max']), (1, needed, maximum))
                self.assertEqual(recipe['slots'], {'upgrades': 1})
                self.assertTrue(recipe['allow_crystal'])
