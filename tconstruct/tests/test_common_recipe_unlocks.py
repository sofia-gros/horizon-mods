"""Source Basalt Blast Furnace and Cheese crafting unlocks."""
import copy
import hashlib
import json
from pathlib import Path
import sys
import tempfile
import unittest
from zipfile import ZipFile

MOD = Path(__file__).resolve().parents[1]
DATA = MOD / 'content/data/tconstruct'
FIXTURES = MOD / 'tests/fixtures/common_recipe_unlocks'
CASES = {
    'basalt_blast_furnace': ('building_blocks', 'has_smooth_stone', 'minecraft:smooth_basalt'),
    'cheese_block_from_ingot': ('food', 'has_cheese', 'tconstruct:cheese_ingot'),
    'cheese_ingot_from_block': ('food', 'has_cheese', 'tconstruct:cheese_block'),
}
sys.path.insert(0, str(MOD / 'tools'))
from package_hmod import package


class CommonRecipeUnlocks(unittest.TestCase):
    def test_exact_source_predicates_and_or_native_recipe_rewards(self):
        hashes = json.loads((FIXTURES / 'sha256.json').read_text())
        self.assertEqual(set(hashes), set(CASES))
        for name, (group, criterion, ingredient) in CASES.items():
            with self.subTest(recipe=name):
                raw = (FIXTURES / (name + '.json')).read_bytes()
                self.assertEqual(hashlib.sha256(raw).hexdigest(), hashes[name])
                expected = copy.deepcopy(json.loads(raw))
                expected['criteria']['has_the_recipe']['conditions'] = {'recipes': 'tconstruct:' + name}
                expected['rewards']['recipes'] = ['tconstruct:' + name]
                native = json.loads((DATA / f'advancement/recipes/{group}/{name}.json').read_text())
                self.assertEqual(native, expected)
                self.assertEqual(native['criteria'][criterion], {
                    'trigger': 'minecraft:inventory_changed',
                    'conditions': {'items': [{'items': [ingredient]}]}})
                self.assertEqual(native['requirements'], [[criterion, 'has_the_recipe']])
                self.assertEqual(native['parent'], 'minecraft:recipes/root')
                self.assertNotIn('display', native)

    def test_existing_native_recipe_inputs_results_and_counts(self):
        with ZipFile(MOD / 'content/reference/tconstruct-recipes.zip') as source:
            for name in CASES:
                with self.subTest(recipe=name):
                    native = json.loads((DATA / f'recipe/{name}.json').read_text())
                    original = json.loads(source.read(f'data/tconstruct/recipes/common/{name}.json'))
                    self.assertEqual(native['type'], original['type'])
                    self.assertEqual(native['result']['id'], original['result']['item'])
                    self.assertEqual(native['result'].get('count', 1), original['result'].get('count', 1))
                    if 'pattern' in original:
                        self.assertEqual(native['pattern'], original['pattern'])
                        self.assertEqual(native['key'], {key: value['item'] for key, value in original['key'].items()})
                    else:
                        self.assertEqual(native['ingredients'], [value['item'] for value in original['ingredients']])

    def test_package_contains_three_unlocks_and_native_recipes(self):
        with tempfile.TemporaryDirectory() as tmp:
            output = Path(tmp) / 'tconstruct.hmod'
            package(output)
            with ZipFile(output) as archive:
                for name, (group, _, _) in CASES.items():
                    for relative in (f'advancement/recipes/{group}/{name}.json', f'recipe/{name}.json'):
                        self.assertEqual(archive.read('data/tconstruct/' + relative), (DATA / relative).read_bytes())


if __name__ == '__main__':
    unittest.main()
