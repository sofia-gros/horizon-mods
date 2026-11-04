"""Check slimewood slab and stair recipes against pinned source data."""
import json
from pathlib import Path
import unittest
from zipfile import ZipFile

ROOT = Path(__file__).resolve().parents[1]
DATA = ROOT / 'content/data/tconstruct'

class SlimewoodShapes(unittest.TestCase):
    def test_all_source_slab_and_stair_recipes(self):
        with ZipFile(ROOT / 'content/reference/tconstruct-recipes.zip') as source:
            for wood in ('greenheart', 'skyroot', 'bloodshroom', 'enderbark'):
                for shape in ('slab', 'stairs'):
                    with self.subTest(wood=wood, shape=shape):
                        original = json.loads(source.read(f'data/tconstruct/recipes/world/wood/{wood}/{shape}.json'))
                        expected = dict(original)
                        expected.pop('show_notification')
                        expected['key'] = {k: v['item'] for k, v in original['key'].items()}
                        expected['result'] = {'id': original['result']['item'], 'count': original['result']['count']}
                        actual = json.loads((DATA / f'recipe/{wood}_planks_{shape}.json').read_text())
                        self.assertEqual(actual, expected)
                        self.assertEqual(actual['result']['count'], 6 if shape == 'slab' else 4)
                        for item in (*actual['key'].values(), actual['result']['id']):
                            self.assertTrue((DATA / f'horizon/block/{item.split(":", 1)[1]}.json').is_file(), item)

if __name__ == '__main__':
    unittest.main()
