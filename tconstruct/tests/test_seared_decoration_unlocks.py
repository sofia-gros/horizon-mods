"""Source criteria for existing Seared construction and supplemental machines."""
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
FIXTURES = MOD / 'tests/fixtures/seared_decoration_unlocks'
GROUPS = {
    'building_blocks': (
        'seared_bricks_from_brick', 'seared_bricks_slab', 'seared_bricks_stairs',
        'seared_bricks_wall', 'seared_cobble_slab', 'seared_cobble_stairs',
        'seared_cobble_wall', 'seared_fancy_bricks_crafting', 'seared_glass',
        'seared_ladder', 'seared_paver_slab', 'seared_paver_stairs',
        'seared_soul_glass', 'seared_stone_slab', 'seared_stone_stairs',
        'seared_tinted_glass'),
    'decorations': ('lantern', 'seared_glass_pane', 'seared_soul_glass_pane'),
    'redstone': ('seared_casting_tank',),
}
sys.path.insert(0, str(MOD / 'tools'))
from package_hmod import package


def native_id(name):
    if name in ('seared_bricks_from_brick', 'seared_glass'):
        return 'survival_seared_' + name
    return 'seared_lantern' if name == 'lantern' else name


def ingredient(name):
    if name == 'seared_fancy_bricks_crafting':
        return 'seared_bricks_slab'
    for block in ('seared_bricks', 'seared_cobble', 'seared_stone', 'seared_paver'):
        if name in tuple(block + '_' + shape for shape in ('slab', 'stairs', 'wall')):
            return block
    if name in ('seared_glass_pane', 'seared_soul_glass_pane'):
        return name.removesuffix('_pane')
    return 'seared_brick'


def cases():
    for group, names in GROUPS.items():
        for name in names:
            yield group, name


class SearedDecorationUnlocks(unittest.TestCase):
    def test_exact_source_inventory_criteria_and_or_recipe_rewards(self):
        hashes = json.loads((FIXTURES / 'sha256.json').read_text())
        self.assertEqual(set(hashes), {name for _, name in cases()})
        self.assertEqual(len(hashes), 20)
        for group, name in cases():
            with self.subTest(recipe=name):
                raw = (FIXTURES / (name + '.json')).read_bytes()
                self.assertEqual(hashlib.sha256(raw).hexdigest(), hashes[name])
                expected = copy.deepcopy(json.loads(raw))
                recipe_id = 'tconstruct:' + native_id(name)
                expected['criteria']['has_the_recipe']['conditions'] = {'recipes': recipe_id}
                expected['rewards']['recipes'] = [recipe_id]
                native = json.loads((DATA / f'advancement/recipes/{group}/{native_id(name)}.json').read_text())
                self.assertEqual(native, expected)
                self.assertEqual(native['criteria']['has_item'], {
                    'trigger': 'minecraft:inventory_changed',
                    'conditions': {'items': [{'items': ['tconstruct:' + ingredient(name)]}]}})
                self.assertEqual(native['requirements'], [['has_item', 'has_the_recipe']])
                self.assertEqual(native['parent'], 'minecraft:recipes/root')
                self.assertNotIn('display', native)

    def test_rewarded_native_patterns_results_and_registered_outputs(self):
        with ZipFile(MOD / 'content/reference/tconstruct-recipes.zip') as source:
            for _, name in cases():
                with self.subTest(recipe=name):
                    native = json.loads((DATA / f'recipe/{native_id(name)}.json').read_text())
                    original = json.loads(source.read(f'data/tconstruct/recipes/smeltery/seared/{name}.json'))
                    self.assertEqual(native['type'], original['type'])
                    self.assertEqual(native['pattern'], original['pattern'])
                    self.assertEqual(native['result']['id'], original['result']['item'])
                    self.assertEqual(native['result'].get('count', 1), original['result'].get('count', 1))
                    output = native['result']['id'].split(':')[1]
                    declaration = json.loads((DATA / f'horizon/block/{output}.json').read_text())
                    self.assertTrue(declaration.get('item', True))

    def test_package_contains_all_twenty_unlocks_and_native_recipes(self):
        with tempfile.TemporaryDirectory() as tmp:
            output = Path(tmp) / 'tconstruct.hmod'
            package(output)
            with ZipFile(output) as archive:
                for group, name in cases():
                    for relative in (f'advancement/recipes/{group}/{native_id(name)}.json',
                                     f'recipe/{native_id(name)}.json'):
                        self.assertEqual(archive.read('data/tconstruct/' + relative), (DATA / relative).read_bytes())


if __name__ == '__main__':
    unittest.main()
