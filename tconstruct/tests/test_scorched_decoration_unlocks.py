"""Pinned unlock criteria for already registered Scorched construction recipes."""
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
FIXTURES = MOD / 'tests/fixtures/scorched_decoration_unlocks'
INGREDIENTS = {
    'chiseled_scorched_bricks_crafting': 'scorched_bricks_slab',
    'polished_scorched_stone_crafting': 'scorched_stone',
    'scorched_bricks_from_brick': 'scorched_brick',
    'scorched_bricks_fence': 'scorched_bricks',
    'scorched_bricks_slab': 'scorched_bricks',
    'scorched_bricks_stairs': 'scorched_bricks',
    'scorched_road_slab': 'scorched_road',
    'scorched_road_stairs': 'scorched_road',
    'scorched_ladder': 'scorched_brick',
    'scorched_soul_glass': 'scorched_brick',
    'scorched_soul_glass_pane': 'scorched_soul_glass',
    'scorched_tinted_glass': 'scorched_brick',
}


def native_id(name):
    return 'survival_scorched_' + name if name == 'scorched_bricks_from_brick' else name


def advancement_path(name):
    group = 'decorations' if name == 'scorched_soul_glass_pane' else 'building_blocks'
    return f'advancement/recipes/{group}/{native_id(name)}.json'


class ScorchedDecorationUnlocks(unittest.TestCase):
    def test_source_inventory_predicates_and_or_recipe_rewards(self):
        hashes = json.loads((FIXTURES / 'sha256.json').read_text())
        self.assertEqual(set(hashes), set(INGREDIENTS))
        self.assertEqual(len(hashes), 12)
        for name, ingredient in INGREDIENTS.items():
            with self.subTest(recipe=name):
                raw = (FIXTURES / (name + '.json')).read_bytes()
                self.assertEqual(hashlib.sha256(raw).hexdigest(), hashes[name])
                expected = copy.deepcopy(json.loads(raw))
                recipe_id = 'tconstruct:' + native_id(name)
                expected['criteria']['has_the_recipe']['conditions'] = {'recipes': recipe_id}
                expected['rewards']['recipes'] = [recipe_id]
                native = json.loads((DATA / advancement_path(name)).read_text())
                self.assertEqual(native, expected)
                self.assertEqual(native['criteria']['has_item'], {
                    'trigger': 'minecraft:inventory_changed',
                    'conditions': {'items': [{'items': ['tconstruct:' + ingredient]}]}})
                self.assertEqual(native['requirements'], [['has_item', 'has_the_recipe']])
                self.assertEqual(native['parent'], 'minecraft:recipes/root')
                self.assertNotIn('display', native)

    def test_existing_native_recipe_ingredients_shapes_counts_and_outputs(self):
        with ZipFile(MOD / 'content/reference/tconstruct-recipes.zip') as source:
            for name in INGREDIENTS:
                with self.subTest(recipe=name):
                    native = json.loads((DATA / f'recipe/{native_id(name)}.json').read_text())
                    original = json.loads(source.read(f'data/tconstruct/recipes/smeltery/scorched/{name}.json'))
                    self.assertEqual(native['type'], original['type'])
                    self.assertEqual(native['pattern'], original['pattern'])
                    expected_keys = {key: value.get('item', '#' + value.get('tag', ''))
                                     for key, value in original['key'].items()}
                    self.assertEqual(native['key'], expected_keys)
                    self.assertEqual(native['result']['id'], original['result']['item'])
                    self.assertEqual(native['result'].get('count', 1), original['result'].get('count', 1))
                    output = native['result']['id'].split(':')[1]
                    declaration = json.loads((DATA / f'horizon/block/{output}.json').read_text())
                    self.assertTrue(declaration.get('item', True))

    def test_package_contains_all_twelve_unlocks_and_native_recipes(self):
        sys.path.insert(0, str(MOD / 'tools'))
        from package_hmod import package
        with tempfile.TemporaryDirectory() as tmp:
            output = Path(tmp) / 'tconstruct.hmod'
            package(output)
            with ZipFile(output) as archive:
                for name in INGREDIENTS:
                    for relative in (advancement_path(name), f'recipe/{native_id(name)}.json'):
                        self.assertEqual(archive.read('data/tconstruct/' + relative), (DATA / relative).read_bytes())


if __name__ == '__main__':
    unittest.main()
