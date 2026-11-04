"""Check smeltery ladder recipes and item tags against pinned source data."""
import json
from pathlib import Path
import unittest
from zipfile import ZipFile

ROOT = Path(__file__).resolve().parents[1]
DATA = ROOT / 'content/data/tconstruct'

class SmelteryLadderRecipes(unittest.TestCase):
    def test_source_tags_and_recipes(self):
        tags = {'seared': 'seared_bricks', 'scorched': 'scorched_blocks'}
        with ZipFile(ROOT / 'content/reference/tconstruct-recipes.zip') as source:
            for material, tag in tags.items():
                with self.subTest(material=material):
                    reference_tag = ROOT / f'content/reference/tconstruct-tags/data/tconstruct/tags/items/{tag}.json'
                    native_tag = json.loads((DATA / f'tags/item/{tag}.json').read_text())
                    self.assertEqual(native_tag, json.loads(reference_tag.read_text()))
                    self.assertFalse(native_tag.get('replace', False))
                    for item in native_tag['values']:
                        self.assertTrue((DATA / f'horizon/block/{item.split(":")[1]}.json').is_file(), item)
                    name = material + '_ladder'
                    original = json.loads(source.read(f'data/tconstruct/recipes/smeltery/{material}/{name}.json'))
                    expected = dict(original)
                    expected.pop('show_notification')
                    expected['key'] = {k: '#' + v['tag'] if 'tag' in v else v['item'] for k, v in original['key'].items()}
                    expected['result'] = {'id': original['result']['item'], 'count': original['result']['count']}
                    actual = json.loads((DATA / f'recipe/{name}.json').read_text())
                    self.assertEqual(actual, expected)
                    self.assertEqual(actual['pattern'], ['b b', 'b b', 'BBB'])
                    self.assertEqual(actual['key'], {'B': f'#tconstruct:{tag}', 'b': f'tconstruct:{material}_brick'})
                    self.assertEqual(actual['result'], {'id': f'tconstruct:{name}', 'count': 4})
                    self.assertTrue((DATA / f'horizon/block/{name}.json').is_file())
                    self.assertIn(f"'tconstruct:{material}_brick'", (ROOT / 'tools/resources.py').read_text())

if __name__ == '__main__':
    unittest.main()
