"""Fourteen direct-item stonecutter routes and their original discovery rules."""
import copy
import hashlib
import json
from pathlib import Path
import sys
import tempfile
import unittest
from zipfile import ZipFile

ROOT = Path(__file__).resolve().parents[1]
DATA = ROOT / 'content/data/tconstruct'
FIXTURES = ROOT / 'tests/fixtures/masonry_stonecutting'
FAMILIES = {
    'seared_bricks': ('slab', 'stairs', 'wall'),
    'seared_cobble': ('slab', 'stairs', 'wall'),
    'seared_paver': ('slab', 'stairs'),
    'seared_stone': ('slab', 'stairs'),
    'scorched_bricks': ('slab', 'stairs'),
    'scorched_road': ('slab', 'stairs'),
}
NAMES = {f'{block}_{shape}_stonecutter' for block, shapes in FAMILIES.items() for shape in shapes}
sys.path.insert(0, str(ROOT / 'tools'))
from package_hmod import package


class MasonryStonecutting(unittest.TestCase):
    def test_all_source_fixtures_are_pinned_and_hashed(self):
        manifest = json.loads((FIXTURES / 'manifest.json').read_text())
        self.assertEqual(len(NAMES), 14)
        self.assertEqual(set(manifest), {f'{kind}/{name}.json'
            for kind in ('recipes', 'advancements') for name in NAMES})
        for relative, entry in manifest.items():
            self.assertEqual(hashlib.sha256((FIXTURES / relative).read_bytes()).hexdigest(), entry['sha256'])
            self.assertIn('/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/', entry['source'])
            self.assertTrue(entry['source'].endswith('/' + Path(relative).name))

    def test_recipe_adaptation_preserves_item_and_count_exactly(self):
        for name in NAMES:
            with self.subTest(name=name):
                source = json.loads((FIXTURES / 'recipes' / (name + '.json')).read_text())
                expected = copy.deepcopy(source)
                self.assertEqual(set(source['ingredient']), {'item'})
                expected['ingredient'] = source['ingredient']['item']
                expected['result'] = {'id': source['result'], 'count': expected.pop('count')}
                actual = json.loads((DATA / 'recipe' / (name + '.json')).read_text())
                self.assertEqual(actual, expected)
                self.assertEqual(actual['type'], 'minecraft:stonecutting')
                self.assertEqual(actual['result']['count'], 2 if '_slab_' in name else 1)
                self.assertEqual(actual['result']['id'], 'tconstruct:' + name.removesuffix('_stonecutter'))
                for item in (actual['ingredient'], actual['result']['id']):
                    self.assertTrue((DATA / 'horizon/block' / (item.split(':', 1)[1] + '.json')).is_file(), item)

    def test_advancements_preserve_source_or_criteria(self):
        for name in NAMES:
            with self.subTest(name=name):
                source = json.loads((FIXTURES / 'advancements' / (name + '.json')).read_text())
                expected = copy.deepcopy(source)
                recipe_id = 'tconstruct:' + name
                expected['criteria']['has_the_recipe']['conditions'] = {'recipes': recipe_id}
                expected['rewards']['recipes'] = [recipe_id]
                actual = json.loads((DATA / 'advancement/recipes/building_blocks' / (name + '.json')).read_text())
                self.assertEqual(actual, expected)
                self.assertEqual(actual['requirements'], [['has_item', 'has_the_recipe']])
                self.assertEqual(actual['parent'], 'minecraft:recipes/root')
                recipe = json.loads((DATA / 'recipe' / (name + '.json')).read_text())
                self.assertEqual(actual['criteria']['has_item'], {'trigger': 'minecraft:inventory_changed',
                    'conditions': {'items': [{'items': [recipe['ingredient']]}]}})
                self.assertEqual(actual['criteria']['has_the_recipe']['trigger'], 'minecraft:recipe_unlocked')
                self.assertNotIn('display', actual)

    def test_crafting_recipes_keep_their_separate_costs_and_ids(self):
        for block, shapes in FAMILIES.items():
            for shape in shapes:
                result = f'{block}_{shape}'
                crafting = json.loads((DATA / 'recipe' / (result + '.json')).read_text())
                self.assertEqual(crafting['type'], 'minecraft:crafting_shaped')
                self.assertEqual(crafting['result']['id'], 'tconstruct:' + result)
                self.assertEqual(crafting['result']['count'], 4 if shape == 'stairs' else 6)
                self.assertNotEqual('tconstruct:' + result, 'tconstruct:' + result + '_stonecutter')

    def test_content_package_has_all_recipe_and_discovery_pairs(self):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / 'tconstruct.hmod'
            package(output)
            with ZipFile(output) as archive:
                for name in NAMES:
                    for kind in ('recipe', 'advancement/recipes/building_blocks'):
                        path = f'data/tconstruct/{kind}/{name}.json'
                        self.assertEqual(archive.read(path), (ROOT / 'content' / path).read_bytes())


if __name__ == '__main__':
    unittest.main()
