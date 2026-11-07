"""Pinned source book/cast unlocks for the existing early crafting recipes."""
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
FIXTURES = MOD / 'tests/fixtures/early_recipe_unlocks'
CASES = {
    'materials_and_you': ('common', 'materials_and_you', 'has_item', 'pattern'),
    'book_substitute': ('tables', 'book_substitute', 'has_item', 'pattern'),
    'sand_cast': ('smeltery', 'blank_sand_cast', 'has_casting', 'seared_table'),
    'red_sand_cast': ('smeltery', 'blank_red_sand_cast', 'has_casting', 'seared_table'),
}


class EarlyRecipeUnlocks(unittest.TestCase):
    def test_source_inventory_conditions_and_or_native_recipe_rewards(self):
        hashes = json.loads((FIXTURES / 'sha256.json').read_text())
        self.assertEqual(set(hashes), set(CASES))
        for name, (_, recipe, criterion, ingredient) in CASES.items():
            with self.subTest(recipe=name):
                raw = (FIXTURES / (name + '.json')).read_bytes()
                self.assertEqual(hashlib.sha256(raw).hexdigest(), hashes[name])
                expected = copy.deepcopy(json.loads(raw))
                expected['criteria']['has_the_recipe']['conditions'] = {'recipes': 'tconstruct:' + recipe}
                expected['rewards']['recipes'] = ['tconstruct:' + recipe]
                native = json.loads((DATA / f'advancement/recipes/misc/{recipe}.json').read_text())
                self.assertEqual(native, expected)
                self.assertEqual(native['criteria'][criterion], {
                    'trigger': 'minecraft:inventory_changed',
                    'conditions': {'items': [{'items': ['tconstruct:' + ingredient]}]}})
                self.assertEqual(native['requirements'], [[criterion, 'has_the_recipe']])
                self.assertEqual(native['parent'], 'minecraft:recipes/root')
                self.assertNotIn('display', native)

    def test_existing_native_inputs_results_and_counts_match_source(self):
        with ZipFile(MOD / 'content/reference/tconstruct-recipes.zip') as source:
            for name, (directory, recipe, _, _) in CASES.items():
                with self.subTest(recipe=name):
                    native = json.loads((DATA / f'recipe/{recipe}.json').read_text())
                    original = json.loads(source.read(f'data/tconstruct/recipes/{directory}/{name}.json'))
                    self.assertEqual(native['type'], original['type'])
                    self.assertEqual(native['ingredients'], [value.get('item', '#' + value.get('tag', ''))
                                                             for value in original['ingredients']])
                    self.assertEqual(native['result']['id'], original['result']['item'])
                    self.assertEqual(native['result'].get('count', 1), original['result'].get('count', 1))

    def test_package_contains_four_unlocks_and_native_recipes(self):
        sys.path.insert(0, str(MOD / 'tools'))
        from package_hmod import package
        with tempfile.TemporaryDirectory() as tmp:
            output = Path(tmp) / 'tconstruct.hmod'
            package(output)
            with ZipFile(output) as archive:
                for _, recipe, _, _ in CASES.values():
                    for relative in (f'advancement/recipes/misc/{recipe}.json', f'recipe/{recipe}.json'):
                        self.assertEqual(archive.read('data/tconstruct/' + relative), (DATA / relative).read_bytes())


if __name__ == '__main__':
    unittest.main()
