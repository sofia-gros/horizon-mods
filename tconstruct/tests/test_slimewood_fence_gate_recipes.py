"""Check slimewood fence gate recipes against pinned source data."""
import json
from pathlib import Path
import unittest
from zipfile import ZipFile

ROOT = Path(__file__).resolve().parents[1]
DATA = ROOT / 'content/data/tconstruct'

class SlimewoodFenceGateRecipes(unittest.TestCase):
    def test_source_fence_gate_recipes(self):
        with ZipFile(ROOT / 'content/reference/tconstruct-recipes.zip') as source:
            for wood in ('greenheart', 'skyroot', 'bloodshroom', 'enderbark'):
                with self.subTest(wood=wood):
                    original = json.loads(source.read(f'data/tconstruct/recipes/world/wood/{wood}/fence_gate.json'))
                    expected = dict(original)
                    expected.pop('show_notification', None)
                    expected['key'] = {k: v['item'] for k, v in original['key'].items()}
                    expected['result'] = {'id': original['result']['item']}
                    actual = json.loads((DATA / f'recipe/{wood}_fence_gate.json').read_text())
                    self.assertEqual(actual, expected)
                    self.assertEqual(actual['pattern'], ['#W#', '#W#'])
                    self.assertEqual(actual['key'], {'#': 'minecraft:stick', 'W': f'tconstruct:{wood}_planks'})
                    for item in (actual['key']['W'], actual['result']['id']):
                        self.assertTrue((DATA / f'horizon/block/{item.split(":", 1)[1]}.json').is_file(), item)

if __name__ == '__main__':
    unittest.main()
