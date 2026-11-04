"""Source-shaped survival acquisition for both already registered lamps."""
import json
import sys
import tempfile
from pathlib import Path
import unittest
from zipfile import ZipFile

ROOT = Path(__file__).resolve().parents[1]
DATA = ROOT / 'content/data/tconstruct'
sys.path.insert(0, str(ROOT / 'tools'))
from package_hmod import package

class SmelteryLampRecipes(unittest.TestCase):
    def test_source_ingredients_and_registered_results(self):
        with ZipFile(ROOT / 'content/reference/tconstruct-recipes.zip') as source:
            for kind in ('seared', 'scorched'):
                with self.subTest(kind=kind):
                    name = f'{kind}_lamp'
                    original = json.loads(source.read(f'data/tconstruct/recipes/smeltery/{kind}/{name}.json'))
                    expected = dict(original)
                    expected.pop('show_notification')
                    expected['key'] = {key: value['item'] for key, value in original['key'].items()}
                    expected['result'] = {'id': original['result']['item']}
                    actual = json.loads((DATA / f'recipe/{name}.json').read_text())
                    self.assertEqual(actual, expected)
                    self.assertEqual(actual['pattern'], [' b ', 'bGb', ' b '])
                    self.assertEqual(actual['key'], {'G': 'minecraft:glowstone', 'b': f'tconstruct:{kind}_brick'})
                    self.assertTrue((DATA / f'horizon/block/{name}.json').is_file())
                    self.assertTrue((DATA / f'loot_table/blocks/{name}.json').is_file())

    def test_recipe_book_unlock_matches_source_conditions(self):
        for kind in ('seared', 'scorched'):
            name = f'{kind}_lamp'
            advancement = json.loads((DATA / f'advancement/recipes/building_blocks/{name}.json').read_text())
            self.assertEqual(advancement['parent'], 'minecraft:recipes/root')
            self.assertEqual(advancement['requirements'], [['has_item', 'has_the_recipe']])
            self.assertEqual(advancement['criteria']['has_item'], {
                'trigger': 'minecraft:inventory_changed',
                'conditions': {'items': [{'items': ['minecraft:glowstone']}]},
            })
            self.assertEqual(advancement['criteria']['has_the_recipe'], {
                'trigger': 'minecraft:recipe_unlocked',
                'conditions': {'recipes': f'tconstruct:{name}'},
            })
            self.assertEqual(advancement['rewards']['recipes'], [f'tconstruct:{name}'])

    def test_native_package_contains_both_recipes(self):
        with tempfile.TemporaryDirectory() as tmp:
            output = Path(tmp) / 'tconstruct.hmod'
            package(output)
            with ZipFile(output) as packed:
                for kind in ('seared', 'scorched'):
                    path = f'data/tconstruct/recipe/{kind}_lamp.json'
                    self.assertEqual(json.loads(packed.read(path)), json.loads((ROOT / 'content' / path).read_text()))
                    path = f'data/tconstruct/advancement/recipes/building_blocks/{kind}_lamp.json'
                    self.assertEqual(json.loads(packed.read(path)), json.loads((ROOT / 'content' / path).read_text()))

if __name__ == '__main__':
    unittest.main()
