"""Check the clear glass pane recipe against pinned source data."""
import json
from pathlib import Path
import unittest
from zipfile import ZipFile

ROOT = Path(__file__).resolve().parents[1]
DATA = ROOT / 'content/data/tconstruct'

class ClearGlassPaneRecipe(unittest.TestCase):
    def test_source_recipe_and_blocks(self):
        with ZipFile(ROOT / 'content/reference/tconstruct-recipes.zip') as source:
            original = json.loads(source.read('data/tconstruct/recipes/common/glass/clear_glass_pane.json'))
        expected = dict(original)
        expected.pop('show_notification')
        expected['key'] = {key: value['item'] for key, value in original['key'].items()}
        expected['result'] = {'id': original['result']['item'], 'count': original['result']['count']}
        actual = json.loads((DATA / 'recipe/clear_glass_pane.json').read_text())
        self.assertEqual(actual, expected)
        self.assertEqual(actual['pattern'], ['###', '###'])
        self.assertEqual(actual['key'], {'#': 'tconstruct:clear_glass'})
        self.assertEqual(actual['result'], {'id': 'tconstruct:clear_glass_pane', 'count': 16})
        for name in ('clear_glass', 'clear_glass_pane'):
            self.assertTrue((DATA / f'horizon/block/{name}.json').is_file())

if __name__ == '__main__':
    unittest.main()
