"""Check stained clear-glass recipes and dye tags against pinned sources."""
import json
from pathlib import Path
import unittest
from zipfile import ZipFile

ROOT = Path(__file__).resolve().parents[1]
DATA = ROOT / 'content/data'
COLORS = ('white', 'orange', 'magenta', 'light_blue', 'yellow', 'lime', 'pink', 'gray', 'light_gray', 'cyan', 'purple', 'blue', 'brown', 'green', 'red', 'black')

class StainedClearGlassRecipes(unittest.TestCase):
    def test_source_recipes_tags_and_blocks(self):
        with ZipFile(ROOT / 'content/reference/tconstruct-recipes.zip') as source, ZipFile(ROOT / 'content/reference/forge-47.4.0-data.zip') as forge:
            for color in COLORS:
                with self.subTest(color=color):
                    tag = json.loads(forge.read(f'data/forge/tags/items/dyes/{color}.json'))
                    actual_tag = json.loads((DATA / f'forge/tags/item/dyes/{color}.json').read_text())
                    self.assertEqual(actual_tag, tag)
                    self.assertFalse(actual_tag.get('replace', False))
                    name = f'{color}_clear_stained_glass'
                    original = json.loads(source.read(f'data/tconstruct/recipes/common/glass/{name}.json'))
                    expected = dict(original)
                    expected.pop('show_notification')
                    expected['key'] = {key: ('#' + value['tag'] if 'tag' in value else value['item']) for key, value in original['key'].items()}
                    expected['result'] = {'id': original['result']['item'], 'count': original['result']['count']}
                    actual = json.loads((DATA / f'tconstruct/recipe/{name}.json').read_text())
                    self.assertEqual(actual, expected)
                    self.assertEqual(actual['pattern'], ['###', '#X#', '###'])
                    self.assertEqual(actual['key'], {'#': 'tconstruct:clear_glass', 'X': f'#forge:dyes/{color}'})
                    self.assertEqual(actual['result'], {'id': f'tconstruct:{name}', 'count': 8})
                    for item in ('clear_glass', name):
                        self.assertTrue((DATA / f'tconstruct/horizon/block/{item}.json').is_file(), item)

if __name__ == '__main__':
    unittest.main()
