"""Check seared cobble shape recipes against pinned source data."""
import json
from pathlib import Path
import unittest
from zipfile import ZipFile

ROOT = Path(__file__).resolve().parents[1]
DATA = ROOT / 'content/data/tconstruct'

class SearedCobbleShapes(unittest.TestCase):
    def test_source_recipes_and_blocks(self):
        cases = {'slab': (['BBB'], 6), 'stairs': (['B  ', 'BB ', 'BBB'], 4), 'wall': (['BBB', 'BBB'], 6)}
        with ZipFile(ROOT / 'content/reference/tconstruct-recipes.zip') as source:
            for kind, (pattern, count) in cases.items():
                with self.subTest(kind=kind):
                    name = 'seared_cobble_' + kind
                    original = json.loads(source.read(f'data/tconstruct/recipes/smeltery/seared/{name}.json'))
                    expected = dict(original)
                    expected.pop('show_notification')
                    expected['key'] = {key: value['item'] for key, value in original['key'].items()}
                    expected['result'] = {'id': original['result']['item'], 'count': original['result']['count']}
                    actual = json.loads((DATA / f'recipe/{name}.json').read_text())
                    self.assertEqual(actual, expected)
                    self.assertEqual(actual['pattern'], pattern)
                    self.assertEqual(actual['key'], {'B': 'tconstruct:seared_cobble'})
                    self.assertEqual(actual['result'], {'id': f'tconstruct:{name}', 'count': count})
                    self.assertTrue((DATA / f'horizon/block/{name}.json').is_file())
        self.assertTrue((DATA / 'horizon/block/seared_cobble.json').is_file())

if __name__ == '__main__':
    unittest.main()
