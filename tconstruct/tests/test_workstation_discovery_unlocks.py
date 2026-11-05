"""Source Chest discovery and bounded Worktable variant recipe rewards."""
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
FIXTURES = MOD / 'tests/fixtures/workstation_discovery_unlocks'
VARIANTS = ('andesite', 'basalt', 'blackstone', 'calcite', 'cobbled_deepslate',
            'cobblestone', 'deepslate', 'diorite', 'dripstone_block', 'granite',
            'mossy_cobblestone', 'stone', 'tuff')
sys.path.insert(0, str(MOD / 'tools'))
from package_hmod import package


class WorkstationDiscoveryUnlocks(unittest.TestCase):
    def source(self, name):
        raw = (FIXTURES / (name + '.json')).read_bytes()
        hashes = json.loads((FIXTURES / 'sha256.json').read_text())
        self.assertEqual(hashlib.sha256(raw).hexdigest(), hashes[name])
        return json.loads(raw)

    def test_chest_preserves_pattern_criterion_and_or_recipe_reward(self):
        expected = self.source('tinkers_chest')
        expected['criteria']['has_the_recipe']['conditions'] = {'recipes': 'tconstruct:tinkers_chest'}
        expected['rewards']['recipes'] = ['tconstruct:tinkers_chest']
        native = json.loads((DATA / 'advancement/recipes/decorations/tinkers_chest.json').read_text())
        self.assertEqual(native, expected)
        self.assertEqual(native['criteria']['has_item']['conditions'], {'items': [{'items': ['tconstruct:pattern']}]})
        self.assertEqual(native['requirements'], [['has_item', 'has_the_recipe']])

    def test_worktable_preserves_source_tag_and_unlocks_all_supported_variants(self):
        original = self.source('modifier_worktable')
        self.assertEqual(original['criteria']['has_item']['conditions'], {'items': [{'tag': 'tconstruct:seared_blocks'}]})
        expected = copy.deepcopy(original)
        expected['criteria']['has_item']['conditions'] = {'items': [{'items': '#tconstruct:seared_blocks'}]}
        del expected['criteria']['has_the_recipe']
        names = ['modifier_worktable_' + variant for variant in VARIANTS]
        criteria = []
        for variant, name in zip(VARIANTS, names):
            criterion = 'has_recipe_' + variant
            expected['criteria'][criterion] = {'trigger': 'minecraft:recipe_unlocked',
                                               'conditions': {'recipes': 'tconstruct:' + name}}
            criteria.append(criterion)
        expected['requirements'] = [['has_item', *criteria]]
        expected['rewards']['recipes'] = ['tconstruct:' + name for name in names]
        native = json.loads((DATA / 'advancement/recipes/decorations/modifier_worktable.json').read_text())
        self.assertEqual(native, expected)
        self.assertEqual({path.stem for path in (DATA / 'recipe').glob('modifier_worktable_*.json')}, set(names))
        self.assertIn('"tconstruct:seared_blocks"', (MOD / 'mixins/tconstruct/source-advancement-tag.rs').read_text())

    def test_existing_crafting_shapes_counts_and_worktable_variant_metadata(self):
        with ZipFile(MOD / 'content/reference/tconstruct-recipes.zip') as source:
            original = json.loads(source.read('data/tconstruct/recipes/tables/tinkers_chest.json'))
            chest = json.loads((DATA / 'recipe/tinkers_chest.json').read_text())
            self.assertEqual(chest['type'], original['type'])
            self.assertEqual(chest['pattern'], [row.replace('l', 'x') for row in original['pattern']])
            self.assertEqual(chest['result']['id'], original['result']['item'])
            self.assertEqual(chest['result'].get('count', 1), original['result'].get('count', 1))
            original = json.loads(source.read('data/tconstruct/recipes/tables/modifier_worktable.json'))
            for variant in VARIANTS:
                native = json.loads((DATA / f'recipe/modifier_worktable_{variant}.json').read_text())
                self.assertEqual(native['type'], 'minecraft:crafting_shaped')
                self.assertEqual(native['pattern'], original['pattern'])
                self.assertEqual(native['key'], {'s': '#tconstruct:seared_blocks', 'r': 'minecraft:' + variant})
                self.assertEqual(native['result']['id'], original['result']['item'])
                self.assertEqual(native['result'].get('count', 1), 1)
                metadata = native['result']['components']['minecraft:custom_data']['tconstruct:anvil']
                self.assertEqual(metadata['Rock'], 'minecraft:' + variant)
                self.assertEqual(metadata['Variant'], variant)

    def test_package_contains_two_unlocks_and_fourteen_native_recipes(self):
        with tempfile.TemporaryDirectory() as tmp:
            output = Path(tmp) / 'tconstruct.hmod'
            package(output)
            with ZipFile(output) as archive:
                paths = ['advancement/recipes/decorations/tinkers_chest.json',
                         'advancement/recipes/decorations/modifier_worktable.json',
                         'recipe/tinkers_chest.json']
                paths.extend(f'recipe/modifier_worktable_{variant}.json' for variant in VARIANTS)
                for relative in paths:
                    self.assertEqual(archive.read('data/tconstruct/' + relative), (DATA / relative).read_bytes())


if __name__ == '__main__':
    unittest.main()
