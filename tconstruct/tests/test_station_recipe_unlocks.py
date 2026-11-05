"""Pinned source Pattern-triggered workstation recipe discovery."""
import json
import sys
import tempfile
import unittest
import zipfile
from pathlib import Path

MOD = Path(__file__).resolve().parents[1]
DATA = MOD / 'content/data/tconstruct'
NAMES = ('crafting_station', 'part_builder', 'tinker_station', 'part_chest')
sys.path.insert(0, str(MOD / 'tools'))
from package_hmod import package


def read(path):
    return json.loads((DATA / path).read_text())


class StationRecipeUnlocks(unittest.TestCase):
    def test_source_inventory_or_recipe_criterion_and_native_rewards(self):
        for name in NAMES:
            with self.subTest(name=name):
                native = 'tconstruct:' + name
                self.assertEqual(read(f'advancement/recipes/decorations/{name}.json'), {
                    'parent': 'minecraft:recipes/root',
                    'criteria': {
                        'has_item': {'conditions': {'items': [{'items': ['tconstruct:pattern']}]},
                                     'trigger': 'minecraft:inventory_changed'},
                        'has_the_recipe': {'conditions': {'recipes': native},
                                           'trigger': 'minecraft:recipe_unlocked'},
                    },
                    'requirements': [['has_item', 'has_the_recipe']],
                    'rewards': {'recipes': [native]},
                    'sends_telemetry_event': False,
                })

    def test_rewards_existing_source_workstation_recipes(self):
        with zipfile.ZipFile(MOD / 'content/reference/tconstruct-recipes.zip') as source:
            for name in NAMES:
                with self.subTest(name=name):
                    original = json.loads(source.read(f'data/tconstruct/recipes/tables/{name}.json'))
                    native = read(f'recipe/{name}.json')
                    self.assertEqual(native['type'], 'minecraft:crafting_shaped')
                    # Source wood-building recipes wrap the shaped recipe.
                    original = original.get('recipe', original)
                    # Existing native Part Chest uses x instead of s for wooden rods.
                    expected_pattern = [row.replace('s', 'x') for row in original['pattern']] if name == 'part_chest' else original['pattern']
                    self.assertEqual(native['pattern'], expected_pattern)
                    self.assertEqual(native['result']['id'], original['result']['item'])
                    self.assertEqual(native['result'].get('count', 1), original['result'].get('count', 1))
                    self.assertIn('tconstruct:pattern', native['key'].values())

    def test_package_includes_unlocks_existing_recipes_and_registered_targets(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'tconstruct.hmod'
            package(path)
            with zipfile.ZipFile(path) as archive:
                for name in NAMES:
                    for relative in (f'advancement/recipes/decorations/{name}.json', f'recipe/{name}.json'):
                        self.assertEqual(json.loads(archive.read('data/tconstruct/' + relative)), read(relative))
                    declaration = json.loads(archive.read(f'data/tconstruct/horizon/block/{name}.json'))
                    self.assertTrue(declaration.get('item', True))
                self.assertIsInstance(json.loads(archive.read('data/tconstruct/horizon/item/pattern.json')), dict)
                for name in ('part_builder', 'tinker_station'):
                    goal = json.loads(archive.read(f'data/tconstruct/advancement/tools/{name}.json'))
                    self.assertNotEqual(goal['parent'], 'minecraft:recipes/root')


if __name__ == '__main__':
    unittest.main()
