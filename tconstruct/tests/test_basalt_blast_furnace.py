"""Check the smooth-basalt blast furnace recipe against pinned source."""
import json
from pathlib import Path
import unittest
from zipfile import ZipFile

ROOT = Path(__file__).resolve().parents[1]

class BasaltBlastFurnace(unittest.TestCase):
    def test_source_recipe(self):
        with ZipFile(ROOT / 'content/reference/tconstruct-recipes.zip') as source:
            original = json.loads(source.read('data/tconstruct/recipes/common/basalt_blast_furnace.json'))
        expected = dict(original)
        expected.pop('show_notification')
        expected['key'] = {key: value['item'] for key, value in original['key'].items()}
        expected['result'] = {'id': original['result']['item']}
        actual = json.loads((ROOT / 'content/data/tconstruct/recipe/basalt_blast_furnace.json').read_text())
        self.assertEqual(actual, expected)
        self.assertEqual(actual['pattern'], ['III', 'IXI', '###'])
        self.assertEqual(actual['key'], {'#': 'minecraft:smooth_basalt', 'I': 'minecraft:iron_ingot', 'X': 'minecraft:furnace'})
        self.assertEqual(actual['result']['id'], 'minecraft:blast_furnace')

if __name__ == '__main__':
    unittest.main()
