"""The native firewood building recipes preserve pinned source behavior."""
import json
from pathlib import Path
import unittest
from zipfile import ZipFile

ROOT = Path(__file__).resolve().parents[1]
DATA = ROOT / 'content/data/tconstruct'

class FirewoodShapes(unittest.TestCase):
    def test_six_recipes_match_source(self):
        with ZipFile(ROOT / 'content/reference/tconstruct-recipes.zip') as source:
            for wood in ('blazewood', 'nahuatl'):
                for shape in ('fence', 'slab', 'stairs'):
                    with self.subTest(wood=wood, shape=shape):
                        name = f'{wood}_{shape}'
                        original = json.loads(source.read(f'data/tconstruct/recipes/common/firewood/{name}.json'))
                        expected = dict(original)
                        expected.pop('show_notification')
                        expected['key'] = {k: v['item'] for k, v in original['key'].items()}
                        expected['result'] = {'id': original['result']['item'], 'count': original['result']['count']}
                        actual = json.loads((DATA / f'recipe/{name}.json').read_text())
                        self.assertEqual(actual, expected)
                        self.assertTrue((DATA / f'horizon/block/{wood}.json').is_file())
                        self.assertTrue((DATA / f'horizon/block/{name}.json').is_file())

if __name__ == '__main__':
    unittest.main()
