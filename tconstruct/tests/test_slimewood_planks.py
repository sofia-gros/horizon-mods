"""Check the four slimewood log-to-plank conversions against source data."""
import json
from pathlib import Path
import unittest
from zipfile import ZipFile

ROOT = Path(__file__).resolve().parents[1]
DATA = ROOT / 'content/data/tconstruct'

class SlimewoodPlanks(unittest.TestCase):
    def test_source_tags_and_recipes(self):
        with ZipFile(ROOT / 'content/reference/tconstruct-recipes.zip') as source:
            for wood in ('greenheart', 'skyroot', 'bloodshroom', 'enderbark'):
                with self.subTest(wood=wood):
                    reference = json.loads((ROOT / f'content/reference/tconstruct-tags/data/tconstruct/tags/items/{wood}_logs.json').read_text())
                    native = json.loads((DATA / f'tags/item/{wood}_logs.json').read_text())
                    self.assertEqual(native, reference)
                    self.assertEqual(len(native['values']), 4)
                    for item in native['values']:
                        self.assertTrue((DATA / f'horizon/block/{item.split(":", 1)[1]}.json').is_file(), item)
                    original = json.loads(source.read(f'data/tconstruct/recipes/world/wood/{wood}/planks.json'))
                    expected = dict(original)
                    expected['ingredients'] = ['#' + item['tag'] for item in original['ingredients']]
                    expected['result'] = {'id': original['result']['item'], 'count': original['result']['count']}
                    self.assertEqual(json.loads((DATA / f'recipe/{wood}_planks.json').read_text()), expected)
                    self.assertEqual(expected['result']['count'], 4)
                    self.assertTrue((DATA / f'horizon/block/{wood}_planks.json').is_file())

if __name__ == '__main__':
    unittest.main()
