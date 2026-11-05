"""Pinned Foundry machinery unlocks mapped to registered native recipes."""
import copy
import hashlib
import json
import sys
import tempfile
import unittest
import zipfile
from pathlib import Path

MOD = Path(__file__).resolve().parents[1]
DATA = MOD / 'content/data/tconstruct'
FIXTURES = MOD / 'tests/fixtures/scorched_recipe_unlocks'
GROUPS = {
    'decorations': ('basin', 'table', 'lantern', 'scorched_glass_pane'),
    'redstone': ('faucet', 'fuel_tank', 'fuel_gauge', 'ingot_tank', 'ingot_gauge', 'drain', 'chute'),
    'building_blocks': ('scorched_glass',),
}
DIRECT = {'basin', 'table', 'lantern', 'faucet', 'scorched_glass_pane'}
sys.path.insert(0, str(MOD / 'tools'))
from package_hmod import package


def native_id(name):
    if name in DIRECT:
        return name if name.startswith('scorched_') else 'scorched_' + name
    return 'survival_scorched_' + name


def path(group, name):
    return f'advancement/recipes/{group}/{native_id(name)}.json'


def adapted(source, name):
    value = copy.deepcopy(source)
    value['criteria']['has_the_recipe']['conditions'] = {'recipes': 'tconstruct:' + native_id(name)}
    value['rewards']['recipes'] = ['tconstruct:' + native_id(name)]
    value['sends_telemetry_event'] = False
    return value


class ScorchedRecipeUnlocks(unittest.TestCase):
    def test_source_conditions_require_brick_or_glass_and_reward_native_recipe(self):
        hashes = json.loads((FIXTURES / 'sha256.json').read_text())
        for group, names in GROUPS.items():
            for name in names:
                with self.subTest(name=name):
                    raw = (FIXTURES / (name + '.json')).read_bytes()
                    self.assertEqual(hashlib.sha256(raw).hexdigest(), hashes[name])
                    source = json.loads(raw)
                    native = json.loads((DATA / path(group, name)).read_text())
                    self.assertEqual(native, adapted(source, name))
                    ingredient = 'scorched_glass' if name == 'scorched_glass_pane' else 'scorched_brick'
                    self.assertEqual(native['criteria']['has_item'], {
                        'trigger': 'minecraft:inventory_changed',
                        'conditions': {'items': [{'items': ['tconstruct:' + ingredient]}]},
                    })
                    self.assertEqual(native['requirements'], [['has_item', 'has_the_recipe']])
                    self.assertEqual(native['parent'], 'minecraft:recipes/root')
                    self.assertNotIn('display', native)

    def test_native_crafting_shapes_outputs_and_counts_match_source(self):
        with zipfile.ZipFile(MOD / 'content/reference/tconstruct-recipes.zip') as archive:
            for names in GROUPS.values():
                for name in names:
                    with self.subTest(name=name):
                        source = json.loads(archive.read(f'data/tconstruct/recipes/smeltery/scorched/{name}.json'))
                        native = json.loads((DATA / f'recipe/{native_id(name)}.json').read_text())
                        self.assertEqual(native['type'], 'minecraft:crafting_shaped')
                        self.assertEqual(native['pattern'], source['pattern'])
                        self.assertEqual(native['result']['id'], source['result']['item'])
                        self.assertEqual(native['result'].get('count', 1), source['result'].get('count', 1))

    def test_package_contains_unlocks_recipes_and_registered_outputs(self):
        with tempfile.TemporaryDirectory() as tmp:
            output = Path(tmp) / 'tconstruct.hmod'
            package(output)
            with zipfile.ZipFile(output) as archive:
                self.assertIsInstance(json.loads(archive.read('data/tconstruct/horizon/item/scorched_brick.json')), dict)
                for group, names in GROUPS.items():
                    for name in names:
                        with self.subTest(name=name):
                            for relative in (path(group, name), f'recipe/{native_id(name)}.json'):
                                self.assertEqual(json.loads(archive.read('data/tconstruct/' + relative)), json.loads((DATA / relative).read_text()))
                            recipe = json.loads((DATA / f'recipe/{native_id(name)}.json').read_text())
                            block_id = recipe['result']['id'].split(':')[1]
                            declaration = json.loads(archive.read(f'data/tconstruct/horizon/block/{block_id}.json'))
                            self.assertTrue(declaration.get('item', True))
