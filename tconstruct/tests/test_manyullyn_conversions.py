"""Check manyullyn conversions and source tags against pinned TConstruct data."""
import json
from pathlib import Path
import unittest
from zipfile import ZipFile

ROOT = Path(__file__).resolve().parents[1]
DATA = ROOT / 'content/data'
NAMES = ('manyullyn_ingot_from_nuggets', 'manyullyn_nugget_from_ingot', 'manyullyn_block_from_ingots', 'manyullyn_ingot_from_block')

class ManyullynConversions(unittest.TestCase):
    def test_source_tags_and_recipes(self):
        for kind in ('ingots', 'nuggets'):
            expected = json.loads((ROOT / f'content/reference/tconstruct-tags/data/forge/tags/items/{kind}/manyullyn.json').read_text())
            actual = json.loads((DATA / f'forge/tags/item/{kind}/manyullyn.json').read_text())
            self.assertEqual(actual, expected)
            self.assertFalse(actual.get('replace', False))
        with ZipFile(ROOT / 'content/reference/tconstruct-recipes.zip') as source:
            for name in NAMES:
                with self.subTest(name=name):
                    original = json.loads(source.read(f'data/tconstruct/recipes/common/materials/{name}.json'))
                    expected = dict(original)
                    expected.pop('show_notification', None)
                    convert = lambda value: '#' + value['tag'] if 'tag' in value else value['item']
                    if 'key' in expected:
                        expected['key'] = {k: convert(v) for k, v in original['key'].items()}
                        self.assertEqual(expected['pattern'], ['###', '#*#', '###'])
                    if 'ingredients' in expected:
                        expected['ingredients'] = [convert(v) for v in original['ingredients']]
                    expected['result'] = {'id': original['result']['item'], **({'count': original['result']['count']} if 'count' in original['result'] else {})}
                    self.assertEqual(json.loads((DATA / f'tconstruct/recipe/{name}.json').read_text()), expected)
        self.assertTrue((DATA / 'tconstruct/horizon/block/manyullyn_block.json').is_file())
        resources = (ROOT / 'tools/resources.py').read_text()
        for item in ('manyullyn_ingot', 'manyullyn_nugget'):
            self.assertIn(f"'tconstruct:{item}'", resources)

if __name__ == '__main__':
    unittest.main()
