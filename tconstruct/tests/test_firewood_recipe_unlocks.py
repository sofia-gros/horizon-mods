"""Source Blazewood/Nahuatl recipe-book criteria for existing native recipes."""
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
FIXTURES = MOD / 'tests/fixtures/firewood_recipe_unlocks'
sys.path.insert(0, str(MOD / 'tools'))
from package_hmod import package


def cases():
    for wood in ('blazewood', 'nahuatl'):
        for shape in ('slab', 'stairs', 'fence'):
            group = 'decorations' if shape == 'fence' else 'building_blocks'
            criterion = 'has_planks' if shape == 'fence' else 'has_item'
            yield wood + '_' + shape, wood, group, criterion


class FirewoodRecipeUnlocks(unittest.TestCase):
    def test_exact_source_item_criteria_and_or_requirements(self):
        hashes = json.loads((FIXTURES / 'sha256.json').read_text())
        self.assertEqual(set(hashes), {name for name, *_ in cases()})
        self.assertEqual(len(hashes), 6)
        for name, wood, group, criterion in cases():
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
                    'conditions': {'items': [{'items': ['tconstruct:' + wood]}]}})
                self.assertEqual(native['requirements'], [[criterion, 'has_the_recipe']])
                self.assertEqual(native['parent'], 'minecraft:recipes/root')
                self.assertNotIn('display', native)

    def test_rewarded_recipe_shapes_ingredients_counts_and_registered_outputs(self):
        with ZipFile(MOD / 'content/reference/tconstruct-recipes.zip') as source:
            for name, *_ in cases():
                with self.subTest(recipe=name):
                    native = json.loads((DATA / f'recipe/{name}.json').read_text())
                    original = json.loads(source.read(f'data/tconstruct/recipes/common/firewood/{name}.json'))
                    self.assertEqual(native['type'], original['type'])
                    self.assertEqual(native['pattern'], original['pattern'])
                    self.assertEqual(native['key'], {key: value['item'] for key, value in original['key'].items()})
                    self.assertEqual(native['result']['id'], original['result']['item'])
                    self.assertEqual(native['result'].get('count', 1), original['result'].get('count', 1))
                    self.assertTrue((DATA / f'horizon/block/{name}.json').is_file())

    def test_package_contains_six_unlocks_and_native_recipes(self):
        with tempfile.TemporaryDirectory() as tmp:
            output = Path(tmp) / 'tconstruct.hmod'
            package(output)
            with ZipFile(output) as archive:
                for name, _, group, _ in cases():
                    for relative in (f'advancement/recipes/{group}/{name}.json', f'recipe/{name}.json'):
                        self.assertEqual(archive.read('data/tconstruct/' + relative), (DATA / relative).read_bytes())


if __name__ == '__main__':
    unittest.main()
