"""Pinned storage/conversion unlocks mapped to existing native metal recipes."""
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
FIXTURES = MOD / 'tests/fixtures/metal_recipe_unlocks'
METALS = ('amethyst_bronze', 'cinderslime', 'cobalt', 'hepatizon', 'knightmetal',
          'knightslime', 'manyullyn', 'pig_iron', 'queens_slime', 'rose_gold',
          'slimesteel', 'steel')
OPERATIONS = {'block_from_ingots': 'ingot', 'ingot_from_block': 'block',
              'ingot_from_nuggets': 'nugget', 'nugget_from_ingot': 'ingot'}
EXTRA = {
    'raw_cobalt_block_from_raws': 'tconstruct:raw_cobalt',
    'raw_cobalt_from_raw_block': 'tconstruct:raw_cobalt_block',
    'netherite_ingot_from_nuggets': 'tconstruct:netherite_nugget',
    'netherite_nugget_from_ingot': 'minecraft:netherite_ingot',
    'debris_nugget_from_ingot': 'minecraft:netherite_scrap',
    'netherite_scrap_from_nuggets': 'tconstruct:debris_nugget',
}
sys.path.insert(0, str(MOD / 'tools'))
from package_hmod import package


def cases():
    for metal in METALS:
        for operation, item in OPERATIONS.items():
            yield metal + '_' + operation, 'tconstruct:' + metal + '_' + item
    yield from EXTRA.items()


def native_ingredient(source):
    return source.get('item', '#' + source.get('tag', ''))


class MetalRecipeUnlocks(unittest.TestCase):
    def test_source_item_predicates_or_requirements_and_native_rewards(self):
        hashes = json.loads((FIXTURES / 'sha256.json').read_text())
        self.assertEqual(set(hashes), {name for name, _ in cases()})
        self.assertEqual(len(hashes), 54)
        for name, ingredient in cases():
            with self.subTest(recipe=name):
                raw = (FIXTURES / (name + '.json')).read_bytes()
                self.assertEqual(hashlib.sha256(raw).hexdigest(), hashes[name])
                expected = copy.deepcopy(json.loads(raw))
                expected['criteria']['has_the_recipe']['conditions'] = {'recipes': 'tconstruct:' + name}
                expected['rewards']['recipes'] = ['tconstruct:' + name]
                native = json.loads((DATA / f'advancement/recipes/misc/{name}.json').read_text())
                self.assertEqual(native, expected)
                self.assertEqual(native['criteria']['has_item'], {
                    'trigger': 'minecraft:inventory_changed',
                    'conditions': {'items': [{'items': [ingredient]}]}})
                self.assertEqual(native['requirements'], [['has_item', 'has_the_recipe']])
                self.assertEqual(native['parent'], 'minecraft:recipes/root')
                self.assertNotIn('display', native)

    def test_native_recipe_shapes_inputs_outputs_and_counts_match_source(self):
        with ZipFile(MOD / 'content/reference/tconstruct-recipes.zip') as source:
            for name, _ in cases():
                with self.subTest(recipe=name):
                    native = json.loads((DATA / f'recipe/{name}.json').read_text())
                    original = json.loads(source.read(f'data/tconstruct/recipes/common/materials/{name}.json'))
                    self.assertEqual(native['type'], original['type'])
                    self.assertEqual(native['result']['id'], original['result']['item'])
                    self.assertEqual(native['result'].get('count', 1), original['result'].get('count', 1))
                    if original['type'] == 'minecraft:crafting_shaped':
                        self.assertEqual(native['pattern'], original['pattern'])
                        self.assertEqual(native['key'], {key: native_ingredient(value)
                                                         for key, value in original['key'].items()})
                    else:
                        self.assertEqual(original['type'], 'minecraft:crafting_shapeless')
                        self.assertEqual(native['ingredients'], [native_ingredient(value)
                                                                 for value in original['ingredients']])

    def test_package_contains_all_54_unlocks_and_native_recipes(self):
        with tempfile.TemporaryDirectory() as tmp:
            output = Path(tmp) / 'tconstruct.hmod'
            package(output)
            with ZipFile(output) as archive:
                for name, _ in cases():
                    for relative in (f'advancement/recipes/misc/{name}.json', f'recipe/{name}.json'):
                        self.assertEqual(archive.read('data/tconstruct/' + relative), (DATA / relative).read_bytes())


if __name__ == '__main__':
    unittest.main()
