"""Check slimewood button and pressure plate recipes against pinned source."""
import json
from pathlib import Path
import unittest
from zipfile import ZipFile

ROOT = Path(__file__).resolve().parents[1]
DATA = ROOT / 'content/data/tconstruct'

class SlimewoodRedstoneRecipes(unittest.TestCase):
    def test_source_button_and_pressure_plate_recipes(self):
        with ZipFile(ROOT / 'content/reference/tconstruct-recipes.zip') as source:
            for wood in ('greenheart', 'skyroot', 'bloodshroom', 'enderbark'):
                for device in ('button', 'pressure_plate'):
                    with self.subTest(wood=wood, device=device):
                        original = json.loads(source.read(f'data/tconstruct/recipes/world/wood/{wood}/{device}.json'))
                        expected = dict(original)
                        expected.pop('show_notification', None)
                        if 'key' in expected:
                            expected['key'] = {k: v['item'] for k, v in original['key'].items()}
                            self.assertEqual(expected['pattern'], ['##'])
                            inputs = list(expected['key'].values())
                        else:
                            expected['ingredients'] = [v['item'] for v in original['ingredients']]
                            inputs = expected['ingredients']
                        expected['result'] = {'id': original['result']['item']}
                        actual = json.loads((DATA / f'recipe/{wood}_{device}.json').read_text())
                        self.assertEqual(actual, expected)
                        self.assertEqual(inputs, [f'tconstruct:{wood}_planks'])
                        for item in (*inputs, actual['result']['id']):
                            self.assertTrue((DATA / f'horizon/block/{item.split(":", 1)[1]}.json').is_file(), item)

if __name__ == '__main__':
    unittest.main()
