"""Pinned Clear Glass recipe-book criteria and existing native recipe mapping."""
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
FIXTURES = MOD / 'tests/fixtures/clear_glass_recipe_unlocks'
COLORS = ('white', 'orange', 'magenta', 'light_blue', 'yellow', 'lime', 'pink',
          'gray', 'light_gray', 'cyan', 'purple', 'blue', 'brown', 'green', 'red', 'black')
sys.path.insert(0, str(MOD / 'tools'))
from package_hmod import package


def cases():
    yield 'clear_glass_pane', 'decorations', 'has_block', 'clear_glass'
    for color in COLORS:
        block = color + '_clear_stained_glass'
        yield block, 'building_blocks', 'has_clear_glass', 'clear_glass'
        yield block + '_pane', 'decorations', 'has_block', block
        yield block + '_pane_from_panes', 'decorations', 'has_clear_glass', 'clear_glass_pane'


class ClearGlassRecipeUnlocks(unittest.TestCase):
    def test_all_source_inventory_and_or_conditions_are_preserved(self):
        hashes = json.loads((FIXTURES / 'sha256.json').read_text())
        self.assertEqual(set(hashes), {name for name, *_ in cases()})
        self.assertEqual(len(hashes), 49)
        for name, group, criterion, ingredient in cases():
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
                    'conditions': {'items': [{'items': ['tconstruct:' + ingredient]}]}})
                self.assertEqual(native['requirements'], [[criterion, 'has_the_recipe']])
                self.assertEqual(native['parent'], 'minecraft:recipes/root')
                self.assertNotIn('display', native)
                self.assertTrue((DATA / f'horizon/block/{ingredient}.json').is_file())

    def test_native_recipe_shapes_colors_and_counts_match_source(self):
        with ZipFile(MOD / 'content/reference/tconstruct-recipes.zip') as source:
            for name, *_ in cases():
                with self.subTest(recipe=name):
                    native = json.loads((DATA / f'recipe/{name}.json').read_text())
                    original = json.loads(source.read(f'data/tconstruct/recipes/common/glass/{name}.json'))
                    self.assertEqual(native['type'], original['type'])
                    self.assertEqual(native['pattern'], original['pattern'])
                    expected_keys = {key: value.get('item', '#' + value.get('tag', ''))
                                     for key, value in original['key'].items()}
                    self.assertEqual(native['key'], expected_keys)
                    self.assertEqual(native['result']['id'], original['result']['item'])
                    self.assertEqual(native['result'].get('count', 1), original['result'].get('count', 1))
                    output = native['result']['id'].split(':')[1]
                    self.assertTrue((DATA / f'horizon/block/{output}.json').is_file())

    def test_package_contains_all_49_unlocks_and_recipes(self):
        with tempfile.TemporaryDirectory() as tmp:
            output = Path(tmp) / 'tconstruct.hmod'
            package(output)
            with ZipFile(output) as archive:
                for name, group, *_ in cases():
                    for relative in (f'advancement/recipes/{group}/{name}.json', f'recipe/{name}.json'):
                        self.assertEqual(archive.read('data/tconstruct/' + relative), (DATA / relative).read_bytes())


if __name__ == '__main__':
    unittest.main()
