"""Check stained clear-glass pane recipes against pinned source data."""
import json
from pathlib import Path
import unittest
from zipfile import ZipFile

ROOT = Path(__file__).resolve().parents[1]
DATA = ROOT / 'content/data/tconstruct'
COLORS = ('white', 'orange', 'magenta', 'light_blue', 'yellow', 'lime', 'pink', 'gray', 'light_gray', 'cyan', 'purple', 'blue', 'brown', 'green', 'red', 'black')

class StainedClearGlassPaneRecipes(unittest.TestCase):
    def test_source_recipes_and_blocks(self):
        with ZipFile(ROOT / 'content/reference/tconstruct-recipes.zip') as source:
            for color in COLORS:
                with self.subTest(color=color):
                    name = f'{color}_clear_stained_glass_pane'
                    original = json.loads(source.read(f'data/tconstruct/recipes/common/glass/{name}.json'))
                    expected = dict(original)
                    expected.pop('show_notification')
                    expected['key'] = {key: value['item'] for key, value in original['key'].items()}
                    expected['result'] = {'id': original['result']['item'], 'count': original['result']['count']}
                    actual = json.loads((DATA / f'recipe/{name}.json').read_text())
                    self.assertEqual(actual, expected)
                    self.assertEqual(actual['pattern'], ['###', '###'])
                    self.assertEqual(actual['key'], {'#': f'tconstruct:{color}_clear_stained_glass'})
                    self.assertEqual(actual['result'], {'id': f'tconstruct:{name}', 'count': 16})
                    for item in (f'{color}_clear_stained_glass', name):
                        self.assertTrue((DATA / f'horizon/block/{item}.json').is_file(), item)

if __name__ == '__main__':
    unittest.main()
