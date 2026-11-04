"""Check slimewood door and trapdoor recipes against pinned source."""
import json
from pathlib import Path
import unittest
from zipfile import ZipFile

ROOT = Path(__file__).resolve().parents[1]
DATA = ROOT / 'content/data/tconstruct'

class SlimewoodDoorRecipes(unittest.TestCase):
    def test_source_door_and_trapdoor_recipes(self):
        with ZipFile(ROOT / 'content/reference/tconstruct-recipes.zip') as source:
            for wood in ('greenheart', 'skyroot', 'bloodshroom', 'enderbark'):
                for device in ('door', 'trapdoor'):
                    with self.subTest(wood=wood, device=device):
                        original = json.loads(source.read(f'data/tconstruct/recipes/world/wood/{wood}/{device}.json'))
                        expected = dict(original)
                        expected.pop('show_notification', None)
                        expected['key'] = {k: v['item'] for k, v in original['key'].items()}
                        expected['result'] = {'id': original['result']['item'], 'count': original['result']['count']}
                        actual = json.loads((DATA / f'recipe/{wood}_{device}.json').read_text())
                        self.assertEqual(actual, expected)
                        self.assertEqual(actual['result']['count'], 3 if device == 'door' else 2)
                        self.assertEqual(list(actual['key'].values()), [f'tconstruct:{wood}_planks'])
                        for item in (*actual['key'].values(), actual['result']['id']):
                            self.assertTrue((DATA / f'horizon/block/{item.split(":", 1)[1]}.json').is_file(), item)

if __name__ == '__main__':
    unittest.main()
