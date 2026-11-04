"""Check slimewood fence recipes and their Forge rod tag against source."""
import json
from pathlib import Path
import unittest
from zipfile import ZipFile

ROOT = Path(__file__).resolve().parents[1]
DATA = ROOT / 'content/data'

class SlimewoodFenceRecipes(unittest.TestCase):
    def test_source_fence_recipes_and_rod_tag(self):
        with ZipFile(ROOT / 'content/reference/forge-47.4.0-data.zip') as forge:
            reference = json.loads(forge.read('data/forge/tags/items/rods/wooden.json'))
            native = json.loads((DATA / 'forge/tags/item/rods/wooden.json').read_text())
            self.assertEqual(native, reference)
            self.assertEqual(native['values'], ['minecraft:stick'])
            self.assertFalse(native.get('replace', False))
        with ZipFile(ROOT / 'content/reference/tconstruct-recipes.zip') as source:
            for wood in ('greenheart', 'skyroot', 'bloodshroom', 'enderbark'):
                with self.subTest(wood=wood):
                    original = json.loads(source.read(f'data/tconstruct/recipes/world/wood/{wood}/fence.json'))
                    expected = dict(original)
                    expected.pop('show_notification', None)
                    expected['key'] = {k: ('#' + v['tag'] if 'tag' in v else v['item']) for k, v in original['key'].items()}
                    expected['result'] = {'id': original['result']['item'], 'count': original['result']['count']}
                    actual = json.loads((DATA / f'tconstruct/recipe/{wood}_fence.json').read_text())
                    self.assertEqual(actual, expected)
                    self.assertEqual(actual['pattern'], ['W#W', 'W#W'])
                    self.assertEqual(actual['key'], {'#': '#forge:rods/wooden', 'W': f'tconstruct:{wood}_planks'})
                    self.assertEqual(actual['result']['count'], 3)
                    for item in (actual['key']['W'], actual['result']['id']):
                        self.assertTrue((DATA / f'tconstruct/horizon/block/{item.split(":", 1)[1]}.json').is_file(), item)

if __name__ == '__main__':
    unittest.main()
