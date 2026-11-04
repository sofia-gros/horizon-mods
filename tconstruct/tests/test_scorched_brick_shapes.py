"""Check scorched brick shape recipes against pinned source data."""
import json
from pathlib import Path
import unittest
from zipfile import ZipFile

ROOT = Path(__file__).resolve().parents[1]
DATA = ROOT / 'content/data/tconstruct'

class ScorchedBrickShapes(unittest.TestCase):
    def test_source_recipes_and_blocks(self):
        cases = {'slab': (['BBB'], 6), 'stairs': (['B  ', 'BB ', 'BBB'], 4), 'fence': (['BbB', 'BbB'], 6)}
        with ZipFile(ROOT / 'content/reference/tconstruct-recipes.zip') as source:
            for kind, (pattern, count) in cases.items():
                with self.subTest(kind=kind):
                    name = 'scorched_bricks_' + kind
                    original = json.loads(source.read(f'data/tconstruct/recipes/smeltery/scorched/{name}.json'))
                    expected = dict(original)
                    expected.pop('show_notification')
                    expected['key'] = {key: value['item'] for key, value in original['key'].items()}
                    expected['result'] = {'id': original['result']['item'], 'count': original['result']['count']}
                    actual = json.loads((DATA / f'recipe/{name}.json').read_text())
                    self.assertEqual(actual, expected)
                    self.assertEqual(actual['pattern'], pattern)
                    expected_key = {'B': 'tconstruct:scorched_bricks'}
                    if kind == 'fence':
                        expected_key['b'] = 'tconstruct:scorched_brick'
                    self.assertEqual(actual['key'], expected_key)
                    self.assertEqual(actual['result'], {'id': f'tconstruct:{name}', 'count': count})
                    self.assertTrue((DATA / f'horizon/block/{name}.json').is_file())
        self.assertTrue((DATA / 'horizon/block/scorched_bricks.json').is_file())
        self.assertIn("'tconstruct:scorched_brick'", (ROOT / 'tools/resources.py').read_text())

if __name__ == '__main__':
    unittest.main()
