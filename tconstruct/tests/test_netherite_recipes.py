"""Pin the original modifier recipes and Forge's vanilla ingot tag."""
import json
from pathlib import Path
import unittest
from zipfile import ZipFile

ROOT = Path(__file__).resolve().parents[1]

class NetheriteRecipes(unittest.TestCase):
    def test_original_requirements_and_inputs(self):
        with ZipFile(ROOT / 'content/reference/tconstruct-recipes.zip') as archive:
            def read(path):
                return json.loads(archive.read('data/tconstruct/recipes/tools/modifiers/' + path + '.json'))
            netherite = read('upgrade/netherite')
            unbreakable = read('ability/unbreakable')
            self.assertEqual(netherite['inputs'], [{'item': 'minecraft:netherite_upgrade_smithing_template'}, {'tag': 'forge:ingots/netherite'}])
            self.assertEqual(netherite['slots'], {'upgrades': 1})
            self.assertEqual(unbreakable['inputs'], [{'item': 'minecraft:shulker_shell'}, {'item': 'minecraft:dragon_breath'}, {'item': 'minecraft:shulker_shell'}, {'tag': 'forge:ingots/netherite'}, {'tag': 'forge:ingots/netherite'}])
            self.assertEqual(unbreakable['slots'], {'abilities': 1})
            self.assertTrue(unbreakable['check_trait_level'])
            for recipe in (netherite, unbreakable):
                self.assertEqual(recipe['tools'], {'tag': 'tconstruct:modifiable/durability'})
                self.assertEqual(recipe['level'], 1)
                self.assertTrue(recipe['allow_crystal'])
        with ZipFile(ROOT / 'content/reference/forge-47.4.0-data.zip') as archive:
            ingots = json.loads(archive.read('data/forge/tags/items/ingots/netherite.json'))
            self.assertIn('minecraft:netherite_ingot', ingots['values'])

if __name__ == '__main__':
    unittest.main()
