"""Check seared and scorched pane recipes against pinned source data."""
import json
from pathlib import Path
import unittest
from zipfile import ZipFile

ROOT = Path(__file__).resolve().parents[1]
DATA = ROOT / 'content/data/tconstruct'

class SmelteryGlassPaneRecipes(unittest.TestCase):
    def test_source_recipes_and_blocks(self):
        with ZipFile(ROOT / 'content/reference/tconstruct-recipes.zip') as source:
            for kind in ('seared', 'scorched'):
                with self.subTest(kind=kind):
                    name = kind + '_glass_pane'
                    original = json.loads(source.read(f'data/tconstruct/recipes/smeltery/{kind}/{name}.json'))
                    expected = dict(original)
                    expected.pop('show_notification')
                    expected['key'] = {key: value['item'] for key, value in original['key'].items()}
                    expected['result'] = {'id': original['result']['item'], 'count': original['result']['count']}
                    actual = json.loads((DATA / f'recipe/{name}.json').read_text())
                    self.assertEqual(actual, expected)
                    self.assertEqual(actual['pattern'], ['###', '###'])
                    self.assertEqual(actual['key'], {'#': f'tconstruct:{kind}_glass'})
                    self.assertEqual(actual['result'], {'id': f'tconstruct:{kind}_glass_pane', 'count': 16})
                    for block in (kind + '_glass', name):
                        self.assertTrue((DATA / f'horizon/block/{block}.json').is_file())

if __name__ == '__main__':
    unittest.main()
