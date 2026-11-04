"""Check smeltery soul-glass recipes against pinned source data."""
import json
from pathlib import Path
import unittest
from zipfile import ZipFile

ROOT = Path(__file__).resolve().parents[1]
DATA = ROOT / 'content/data/tconstruct'

class SmelterySoulGlassRecipes(unittest.TestCase):
    def test_source_recipes_and_blocks(self):
        self.assertTrue((DATA / 'horizon/block/soul_glass.json').is_file())
        with ZipFile(ROOT / 'content/reference/tconstruct-recipes.zip') as source:
            for material in ('seared', 'scorched'):
                for pane in (False, True):
                    with self.subTest(material=material, pane=pane):
                        name = material + '_soul_glass' + ('_pane' if pane else '')
                        original = json.loads(source.read(f'data/tconstruct/recipes/smeltery/{material}/{name}.json'))
                        expected = dict(original)
                        expected.pop('show_notification')
                        expected['key'] = {key: value['item'] for key, value in original['key'].items()}
                        expected['result'] = {'id': original['result']['item'], **({'count': original['result']['count']} if 'count' in original['result'] else {})}
                        actual = json.loads((DATA / f'recipe/{name}.json').read_text())
                        self.assertEqual(actual, expected)
                        self.assertEqual(actual['pattern'], ['###', '###'] if pane else [' b ', 'bGb', ' b '])
                        self.assertEqual(actual['key'], {'#': f'tconstruct:{material}_soul_glass'} if pane else {'G': 'tconstruct:soul_glass', 'b': f'tconstruct:{material}_brick'})
                        self.assertEqual(actual['result'], {'id': f'tconstruct:{name}', **({'count': 16} if pane else {})})
                        self.assertTrue((DATA / f'horizon/block/{name}.json').is_file())
        resources = (ROOT / 'tools/resources.py').read_text()
        for material in ('seared', 'scorched'):
            self.assertIn(f"'tconstruct:{material}_brick'", resources)

if __name__ == '__main__':
    unittest.main()
