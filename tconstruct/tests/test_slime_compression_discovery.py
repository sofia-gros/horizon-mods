"""Color-specific compression unlock data; world-tag bridge integration is separate."""
import hashlib
import json
from pathlib import Path
import sys
import tempfile
import unittest
from zipfile import ZipFile

MOD = Path(__file__).resolve().parents[1]
CONTENT = MOD / 'content'
DATA = CONTENT / 'data/tconstruct'
FIXTURE = MOD / 'tests/fixtures/slime_compression_discovery'
KINDS = ('sky', 'ichor', 'ender')
sys.path.insert(0, str(MOD / 'tools'))
from package_hmod import package


class SlimeCompressionDiscovery(unittest.TestCase):
    def test_source_tag_or_recipe_criteria_and_exact_native_reward(self):
        hashes = json.loads((FIXTURE / 'sha256.json').read_text())
        self.assertEqual(set(hashes), set(KINDS))
        for kind in KINDS:
            with self.subTest(kind=kind):
                raw = (FIXTURE / (kind + '.json')).read_bytes()
                self.assertEqual(hashlib.sha256(raw).hexdigest(), hashes[kind])
                expected = json.loads(raw)
                tag = 'forge:slimeball/' + kind
                recipe = 'tconstruct:' + kind + '_slime'
                self.assertEqual(expected['criteria']['has_item']['conditions'], {'items': [{'tag': tag}]})
                self.assertEqual(expected['criteria']['has_the_recipe']['conditions'],
                                 {'recipe': f'tconstruct:common/slime/{kind}/slimeblock'})
                expected['criteria']['has_item']['conditions'] = {'items': [{'items': '#' + tag}]}
                expected['criteria']['has_the_recipe']['conditions'] = {'recipes': recipe}
                expected['rewards']['recipes'] = [recipe]
                native = json.loads((DATA / f'advancement/recipes/redstone/{kind}_slime.json').read_text())
                self.assertEqual(native, expected)
                self.assertEqual(native['requirements'], [['has_item', 'has_the_recipe']])
                self.assertEqual(native['parent'], 'minecraft:recipes/root')
                self.assertNotIn('display', native)

    def test_rewards_reference_existing_nine_ball_recipes_and_stock_color_tags(self):
        for kind in KINDS:
            recipe = json.loads((DATA / f'recipe/{kind}_slime.json').read_text())
            self.assertEqual(recipe['type'], 'minecraft:crafting_shaped')
            self.assertEqual(recipe['pattern'], ['###'] * 3)
            self.assertEqual(recipe['key'], {'#': '#forge:slimeball/' + kind})
            self.assertEqual(recipe['result'], {'id': 'tconstruct:' + kind + '_slime'})
            self.assertTrue((DATA / f'horizon/block/{kind}_slime.json').is_file())
            tag = json.loads((CONTENT / f'data/forge/tags/item/slimeball/{kind}.json').read_text())
            self.assertEqual(tag['values'], ['tconstruct:' + kind + '_slime_ball'])
            recovery = json.loads((DATA / f'advancement/recipes/misc/{kind}_slimeball_from_block.json').read_text())
            self.assertEqual(recovery['rewards']['recipes'], ['tconstruct:' + kind + '_slimeball_from_block'])

    def test_package_contains_three_unlocks_and_existing_compression_recipes(self):
        with tempfile.TemporaryDirectory() as tmp:
            output = Path(tmp) / 'tconstruct.hmod'
            package(output)
            with ZipFile(output) as archive:
                for kind in KINDS:
                    for relative in (f'advancement/recipes/redstone/{kind}_slime.json', f'recipe/{kind}_slime.json'):
                        self.assertEqual(archive.read('data/tconstruct/' + relative), (DATA / relative).read_bytes())


if __name__ == '__main__':
    unittest.main()
