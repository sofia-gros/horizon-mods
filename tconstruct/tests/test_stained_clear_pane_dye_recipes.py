"""Check direct clear-pane dye recipes against pinned source data."""
import json
from pathlib import Path
import unittest
from zipfile import ZipFile

ROOT = Path(__file__).resolve().parents[1]
DATA = ROOT / 'content/data'
COLORS = ('white', 'orange', 'magenta', 'light_blue', 'yellow', 'lime', 'pink', 'gray', 'light_gray', 'cyan', 'purple', 'blue', 'brown', 'green', 'red', 'black')

class StainedClearPaneDyeRecipes(unittest.TestCase):
    def test_source_recipes_and_existing_inputs_outputs(self):
        with ZipFile(ROOT / 'content/reference/tconstruct-recipes.zip') as source, ZipFile(ROOT / 'content/reference/forge-47.4.0-data.zip') as forge:
            for color in COLORS:
                with self.subTest(color=color):
                    name = f'{color}_clear_stained_glass_pane_from_panes'
                    original = json.loads(source.read(f'data/tconstruct/recipes/common/glass/{name}.json'))
                    expected = dict(original)
                    expected.pop('show_notification')
                    expected['key'] = {k: ('#' + v['tag'] if 'tag' in v else v['item']) for k, v in original['key'].items()}
                    expected['result'] = {'id': original['result']['item'], 'count': original['result']['count']}
                    actual = json.loads((DATA / f'tconstruct/recipe/{name}.json').read_text())
                    self.assertEqual(actual, expected)
                    self.assertEqual(actual['pattern'], ['###', '#X#', '###'])
                    self.assertEqual(actual['key'], {'#': 'tconstruct:clear_glass_pane', 'X': f'#forge:dyes/{color}'})
                    self.assertEqual(actual['result'], {'id': f'tconstruct:{color}_clear_stained_glass_pane', 'count': 8})
                    self.assertTrue((DATA / 'tconstruct/horizon/block/clear_glass_pane.json').is_file())
                    self.assertTrue((DATA / f'tconstruct/horizon/block/{color}_clear_stained_glass_pane.json').is_file())
                    self.assertEqual(json.loads((DATA / f'forge/tags/item/dyes/{color}.json').read_text()), json.loads(forge.read(f'data/forge/tags/items/dyes/{color}.json')))

if __name__ == '__main__':
    unittest.main()
