"""Source Gold Platform discovery, without claiming block placement support."""
import hashlib
import json
from pathlib import Path
import sys
import tempfile
import unittest
from zipfile import ZipFile

MOD = Path(__file__).resolve().parents[1]
DATA = MOD / 'content/data/tconstruct'
FIXTURE = MOD / 'tests/fixtures/gold_platform_unlock'
sys.path.insert(0, str(MOD / 'tools'))
from package_hmod import package


class GoldPlatformUnlock(unittest.TestCase):
    def test_source_gold_tag_or_recipe_criterion_and_reward(self):
        raw = (FIXTURE / 'gold_platform.json').read_bytes()
        self.assertEqual(hashlib.sha256(raw).hexdigest(), json.loads((FIXTURE / 'sha256.json').read_text())['gold_platform'])
        expected = json.loads(raw)
        self.assertEqual(expected['criteria']['has_gold']['conditions'], {'items': [{'tag': 'forge:ingots/gold'}]})
        expected['criteria']['has_gold']['conditions'] = {'items': [{'items': '#forge:ingots/gold'}]}
        expected['criteria']['has_the_recipe']['conditions'] = {'recipes': 'tconstruct:gold_platform'}
        expected['rewards']['recipes'] = ['tconstruct:gold_platform']
        native = json.loads((DATA / 'advancement/recipes/decorations/gold_platform.json').read_text())
        self.assertEqual(native, expected)
        self.assertEqual(native['requirements'], [['has_gold', 'has_the_recipe']])
        self.assertNotIn('display', native)
        self.assertIn('"forge:ingots/gold"', (MOD / 'mixins/tconstruct/source-advancement-tag.rs').read_text())

    def test_discovery_keeps_partial_item_recipe_and_placement_warning(self):
        with ZipFile(MOD / 'content/reference/tconstruct-recipes.zip') as source:
            original = json.loads(source.read('data/tconstruct/recipes/common/gold_platform.json'))
        recipe = json.loads((DATA / 'recipe/gold_platform.json').read_text())
        self.assertEqual(recipe['type'], original['type'])
        self.assertEqual(recipe['pattern'], original['pattern'])
        self.assertEqual(recipe['key'], {'#': '#forge:ingots/gold', '.': '#forge:nuggets/gold'})
        self.assertEqual(recipe['result'], {'id': original['result']['item'], 'count': original['result']['count']})
        self.assertEqual(json.loads((DATA / 'horizon/item/gold_platform.json').read_text()), {'template': 'minecraft:gold_ingot'})
        self.assertFalse((DATA / 'horizon/block/gold_platform.json').exists())
        self.assertIn('placement unavailable', (MOD / 'tools/piggy_labels.py').read_text())
        basin = json.loads((DATA / 'recipe/scorched_basin.json').read_text())
        self.assertEqual(basin['key']['G'], 'tconstruct:gold_platform')

    def test_package_contains_advancement_and_existing_recipe(self):
        with tempfile.TemporaryDirectory() as tmp:
            output = Path(tmp) / 'tconstruct.hmod'
            package(output)
            with ZipFile(output) as archive:
                for relative in ('advancement/recipes/decorations/gold_platform.json', 'recipe/gold_platform.json'):
                    self.assertEqual(archive.read('data/tconstruct/' + relative), (DATA / relative).read_bytes())


if __name__ == '__main__':
    unittest.main()
