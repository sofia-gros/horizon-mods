"""Pinned slime crystal recipe-book unlocks, without gameplay claims."""
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
FIXTURES = MOD / 'tests/fixtures/slime_crystal_unlocks'
KINDS = ('earth', 'sky', 'ichor', 'ender')
OPERATIONS = ('crystal_block', 'crystal_growing', 'crystal_smelting')
sys.path.insert(0, str(MOD / 'tools'))
from package_hmod import package


def native_path(kind, operation):
    group = 'building_blocks' if operation == 'crystal_block' else 'misc'
    return f'advancement/recipes/{group}/{kind}_{operation}.json'


class SlimeCrystalUnlocks(unittest.TestCase):
    def test_exact_source_criteria_and_native_rewards(self):
        hashes = json.loads((FIXTURES / 'sha256.json').read_text())
        self.assertEqual(len(hashes), 12)
        for kind in KINDS:
            for operation in OPERATIONS:
                with self.subTest(kind=kind, operation=operation):
                    name = f'{kind}_{operation}'
                    raw = (FIXTURES / (name + '.json')).read_bytes()
                    self.assertEqual(hashlib.sha256(raw).hexdigest(), hashes[name])
                    source = json.loads(raw)
                    expected = copy.deepcopy(source)
                    expected['criteria']['has_the_recipe']['conditions'] = {'recipes': 'tconstruct:' + name}
                    expected['rewards']['recipes'] = ['tconstruct:' + name]
                    native = json.loads((DATA / native_path(kind, operation)).read_text())
                    self.assertEqual(native, expected)
                    criterion = {'crystal_block': 'has_item', 'crystal_growing': 'has_dirt',
                                 'crystal_smelting': 'has_crystal'}[operation]
                    ingredient = f'tconstruct:{kind}_slime_' + ('dirt' if operation == 'crystal_growing' else 'crystal')
                    self.assertEqual(native['criteria'][criterion], {
                        'trigger': 'minecraft:inventory_changed',
                        'conditions': {'items': [{'items': [ingredient]}]}})
                    self.assertEqual(native['requirements'], [[criterion, 'has_the_recipe']])
                    self.assertEqual(native['parent'], 'minecraft:recipes/root')
                    self.assertNotIn('display', native)

    def test_unlocks_point_to_existing_source_recipe_shapes_and_outputs(self):
        with ZipFile(MOD / 'content/reference/tconstruct-recipes.zip') as source:
            for kind in KINDS:
                for operation in OPERATIONS:
                    with self.subTest(kind=kind, operation=operation):
                        native = json.loads((DATA / f'recipe/{kind}_{operation}.json').read_text())
                        original = json.loads(source.read(
                            f'data/tconstruct/recipes/common/slime/{kind}/{operation}.json'))
                        self.assertEqual(native['type'], original['type'])
                        result = original['result']
                        self.assertEqual(native['result']['id'], result if isinstance(result, str) else result['item'])
                        if operation == 'crystal_block':
                            self.assertEqual(native['pattern'], original['pattern'])
                            self.assertEqual(native['key']['#'], original['key']['#']['item'])
                        else:
                            self.assertEqual(native['ingredient'], original['ingredient']['item'])
                            self.assertEqual(native['cookingtime'], original['cookingtime'])
                            self.assertEqual(native['experience'], original['experience'])

    def test_package_contains_all_twelve_unlocks_and_recipes(self):
        with tempfile.TemporaryDirectory() as tmp:
            output = Path(tmp) / 'tconstruct.hmod'
            package(output)
            with ZipFile(output) as archive:
                for kind in KINDS:
                    for operation in OPERATIONS:
                        for relative in (native_path(kind, operation), f'recipe/{kind}_{operation}.json'):
                            self.assertEqual(archive.read('data/tconstruct/' + relative), (DATA / relative).read_bytes())


if __name__ == '__main__':
    unittest.main()
