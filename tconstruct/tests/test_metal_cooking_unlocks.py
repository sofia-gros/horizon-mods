"""Pinned cooking unlocks, including one source recipe split into two natives."""
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
FIXTURES = MOD / 'tests/fixtures/metal_cooking_unlocks'
CASES = {
    'cobalt_ingot_blasting': ('raw_cobalt', ('cobalt_raw_blasting', 'cobalt_ore_blasting')),
    'cobalt_nugget_blasting': ('cobalt_shard', ('cobalt_shard_blasting',)),
    'steel_nugget_smelting': ('steel_shard', ('steel_nugget_smelting',)),
    'steel_nugget_blasting': ('steel_shard', ('steel_nugget_blasting',)),
    'knightmetal_nugget_smelting': ('knightmetal_shard', ('knightmetal_nugget_smelting',)),
    'knightmetal_nugget_blasting': ('knightmetal_shard', ('knightmetal_nugget_blasting',)),
}
sys.path.insert(0, str(MOD / 'tools'))
from package_hmod import package


def advancement_name(source):
    return 'cobalt_shard_blasting' if source == 'cobalt_nugget_blasting' else source


def adapted(source, names):
    expected = copy.deepcopy(source)
    expected['criteria']['has_the_recipe']['conditions'] = {'recipes': 'tconstruct:' + names[0]}
    expected['rewards']['recipes'] = ['tconstruct:' + name for name in names]
    if len(names) == 2:
        expected['criteria']['has_ore_recipe'] = {
            'trigger': 'minecraft:recipe_unlocked',
            'conditions': {'recipes': 'tconstruct:' + names[1]}}
        expected['requirements'] = [['has_item', 'has_the_recipe', 'has_ore_recipe']]
    return expected


class MetalCookingUnlocks(unittest.TestCase):
    def test_source_inventory_criteria_and_mapped_recipe_alternatives(self):
        hashes = json.loads((FIXTURES / 'sha256.json').read_text())
        self.assertEqual(set(hashes), set(CASES))
        for name, (ingredient, names) in CASES.items():
            with self.subTest(recipe=name):
                raw = (FIXTURES / (name + '.json')).read_bytes()
                self.assertEqual(hashlib.sha256(raw).hexdigest(), hashes[name])
                native = json.loads((DATA / f'advancement/recipes/misc/{advancement_name(name)}.json').read_text())
                self.assertEqual(native, adapted(json.loads(raw), names))
                self.assertEqual(native['criteria']['has_item'], {
                    'trigger': 'minecraft:inventory_changed',
                    'conditions': {'items': [{'items': ['tconstruct:' + ingredient]}]}})
                self.assertEqual(native['parent'], 'minecraft:recipes/root')
                self.assertNotIn('display', native)

    def test_seven_native_cooking_recipes_match_source_inputs_results_times_and_xp(self):
        with ZipFile(MOD / 'content/reference/tconstruct-recipes.zip') as source:
            for name, (_, names) in CASES.items():
                original = json.loads(source.read(f'data/tconstruct/recipes/common/materials/{name}.json'))
                source_inputs = original['ingredient']
                if not isinstance(source_inputs, list):
                    source_inputs = [source_inputs]
                self.assertEqual(len(source_inputs), len(names))
                for native_name, source_input in zip(names, source_inputs):
                    with self.subTest(recipe=native_name):
                        native = json.loads((DATA / f'recipe/{native_name}.json').read_text())
                        for key in ('type', 'category', 'cookingtime', 'experience'):
                            self.assertEqual(native[key], original[key])
                        expected_input = source_input.get('item', '#' + source_input.get('tag', ''))
                        self.assertEqual(native['ingredient'], expected_input)
                        self.assertEqual(native['result'], {'id': original['result']})

    def test_package_contains_six_unlocks_and_seven_native_recipes(self):
        with tempfile.TemporaryDirectory() as tmp:
            output = Path(tmp) / 'tconstruct.hmod'
            package(output)
            with ZipFile(output) as archive:
                for name, (_, names) in CASES.items():
                    paths = [f'advancement/recipes/misc/{advancement_name(name)}.json']
                    paths.extend(f'recipe/{native_name}.json' for native_name in names)
                    for relative in paths:
                        self.assertEqual(archive.read('data/tconstruct/' + relative), (DATA / relative).read_bytes())


if __name__ == '__main__':
    unittest.main()
