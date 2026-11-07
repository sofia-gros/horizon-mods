"""Source-exact direct-item recipe-book unlocks for four slimewood families."""
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
FIXTURES = MOD / 'tests/fixtures/slimewood_recipe_unlocks'
WOODS = ('greenheart', 'skyroot', 'bloodshroom', 'enderbark')
GROUPS = {
    'building_blocks': ('log_to_wood', 'stripped_log_to_wood', 'slab', 'stairs'),
    'redstone': ('button', 'pressure_plate', 'door', 'trapdoor', 'fence_gate'),
    'decorations': ('fence',),
}
sys.path.insert(0, str(MOD / 'tools'))
from package_hmod import package


def native_name(wood, name):
    if name == 'log_to_wood':
        return wood + '_wood'
    if name == 'stripped_log_to_wood':
        return 'stripped_' + wood + '_wood'
    if name in ('slab', 'stairs'):
        return wood + '_planks_' + name
    return wood + '_' + name


class SlimewoodRecipeUnlocks(unittest.TestCase):
    def test_forty_source_criteria_and_rewards_preserve_family_and_or(self):
        hashes = json.loads((FIXTURES / 'sha256.json').read_text())
        self.assertEqual(len(hashes), 40)
        for wood in WOODS:
            for group, names in GROUPS.items():
                for name in names:
                    with self.subTest(wood=wood, recipe=name):
                        fixture = wood + '_' + name
                        raw = (FIXTURES / (fixture + '.json')).read_bytes()
                        self.assertEqual(hashlib.sha256(raw).hexdigest(), hashes[fixture])
                        expected = copy.deepcopy(json.loads(raw))
                        recipe = 'tconstruct:' + native_name(wood, name)
                        expected['criteria']['has_the_recipe']['conditions'] = {'recipes': recipe}
                        expected['rewards']['recipes'] = [recipe]
                        path = DATA / f'advancement/recipes/{group}/{native_name(wood, name)}.json'
                        actual = json.loads(path.read_text())
                        self.assertEqual(actual, expected)
                        criterion = 'has_log' if name.endswith('log_to_wood') else 'has_planks'
                        ingredient = ('stripped_' if name.startswith('stripped_') else '') + wood
                        ingredient += '_log' if criterion == 'has_log' else '_planks'
                        self.assertEqual(actual['criteria'][criterion], {
                            'trigger': 'minecraft:inventory_changed',
                            'conditions': {'items': [{'items': ['tconstruct:' + ingredient]}]}})
                        self.assertEqual(actual['requirements'], [[criterion, 'has_the_recipe']])
                        self.assertEqual(actual['parent'], 'minecraft:recipes/root')
                        self.assertNotIn('display', actual)

    def test_existing_recipe_results_and_counts_match_source(self):
        with ZipFile(MOD / 'content/reference/tconstruct-recipes.zip') as source:
            for wood in WOODS:
                for names in GROUPS.values():
                    for name in names:
                        with self.subTest(wood=wood, recipe=name):
                            native = json.loads((DATA / f'recipe/{native_name(wood, name)}.json').read_text())
                            original = json.loads(source.read(f'data/tconstruct/recipes/world/wood/{wood}/{name}.json'))
                            self.assertEqual(native['type'], original['type'])
                            self.assertEqual(native['result']['id'], original['result']['item'])
                            self.assertEqual(native['result'].get('count', 1), original['result'].get('count', 1))
                            if 'pattern' in original:
                                self.assertEqual(native['pattern'], original['pattern'])
                            result = native['result']['id'].split(':')[1]
                            self.assertTrue((DATA / f'horizon/block/{result}.json').is_file())

    def test_package_contains_all_unlocks_and_recipe_endpoints(self):
        with tempfile.TemporaryDirectory() as tmp:
            output = Path(tmp) / 'tconstruct.hmod'
            package(output)
            with ZipFile(output) as archive:
                for wood in WOODS:
                    for group, names in GROUPS.items():
                        for name in names:
                            native = native_name(wood, name)
                            for relative in (f'advancement/recipes/{group}/{native}.json', f'recipe/{native}.json'):
                                self.assertEqual(archive.read('data/tconstruct/' + relative), (DATA / relative).read_bytes())


if __name__ == '__main__':
    unittest.main()
