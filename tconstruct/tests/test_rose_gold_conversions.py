"""Rose Gold crafting conversions preserve pinned source shape and tags."""
import json
from pathlib import Path
import unittest
from zipfile import ZipFile

ROOT = Path(__file__).resolve().parents[1]
DATA = ROOT / 'content/data'
REFERENCE_TAGS = ROOT / 'content/reference/tconstruct-tags/data/forge/tags/items'

class RoseGoldConversions(unittest.TestCase):
    def test_adapted_recipes_match_source(self):
        kinds = ('block_from_ingots', 'ingot_from_block', 'ingot_from_nuggets', 'nugget_from_ingot')
        with ZipFile(ROOT / 'content/reference/tconstruct-recipes.zip') as archive:
            for kind in kinds:
                name = 'rose_gold_' + kind
                with self.subTest(name=name):
                    source = json.loads(archive.read('data/tconstruct/recipes/common/materials/' + name + '.json'))
                    actual = json.loads((DATA / 'tconstruct/recipe' / (name + '.json')).read_text())
                    self.assertEqual(actual['type'], source['type'])
                    self.assertEqual(actual['category'], source['category'])
                    if 'pattern' in source:
                        self.assertEqual(actual['pattern'], source['pattern'])
                        self.assertEqual(actual['key'], {k: '#' + v['tag'] if 'tag' in v else v['item'] for k, v in source['key'].items()})
                    else:
                        self.assertEqual(actual['ingredients'], [v['item'] for v in source['ingredients']])
                    self.assertEqual(actual['result'], {'id': source['result']['item'], **({'count': source['result']['count']} if source['result'].get('count', 1) != 1 else {})})

    def test_native_tags_match_source(self):
        for category in ('ingots', 'nuggets'):
            self.assertEqual(json.loads((DATA / 'forge/tags/item' / category / 'rose_gold.json').read_text()),
                             json.loads((REFERENCE_TAGS / category / 'rose_gold.json').read_text()))

if __name__ == '__main__':
    unittest.main()
