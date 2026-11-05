"""Crafting Station acquisition retains the source live-tag-minus-self rule."""
import copy
import hashlib
import json
from pathlib import Path
import sys
import tempfile
import unittest
from zipfile import ZipFile

ROOT = Path(__file__).resolve().parents[1]
DATA = ROOT / 'content/data'
FIXTURES = ROOT / 'tests/fixtures/crafting_station_workbench'
sys.path.insert(0, str(ROOT / 'tools'))
from package_hmod import package


class CraftingStationWorkbenchInput(unittest.TestCase):
    def test_pinned_source_fixture_hashes(self):
        manifest = json.loads((FIXTURES / 'manifest.json').read_text())
        self.assertEqual(set(manifest), {'recipe.json', 'advancement.json', 'tag.json'})
        for name, entry in manifest.items():
            self.assertEqual(hashlib.sha256((FIXTURES / name).read_bytes()).hexdigest(), entry['sha256'])
            self.assertIn('/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/', entry['source'])

    def test_only_source_ingredient_and_result_syntax_are_adapted(self):
        source = json.loads((FIXTURES / 'recipe.json').read_text())
        self.assertEqual(source['key']['w'], {'type': 'forge:difference',
            'base': {'tag': 'forge:workbenches'}, 'subtracted': {'item': 'tconstruct:crafting_station'}})
        expected = copy.deepcopy(source)
        expected['key'] = {'p': source['key']['p']['item'], 'w': {'type': 'neoforge:difference',
            'base': '#' + source['key']['w']['base']['tag'], 'subtracted': source['key']['w']['subtracted']['item']}}
        expected['result'] = {'id': source['result']['item']}
        native = json.loads((DATA / 'tconstruct/recipe/crafting_station.json').read_text())
        self.assertEqual(native, expected)
        self.assertEqual(native['pattern'], ['p', 'w'])
        self.assertEqual(native['result'], {'id': 'tconstruct:crafting_station'})
        self.assertTrue(native['show_notification'])
        self.assertNotIn('source_forge_workbenches', json.dumps(native))

    def test_live_workbench_tag_keeps_source_optional_legacy_membership(self):
        source = json.loads((FIXTURES / 'tag.json').read_text())
        native = json.loads((DATA / 'forge/tags/item/workbenches.json').read_text())
        self.assertEqual(native, source)
        self.assertEqual(native, {'values': ['minecraft:crafting_table', 'tconstruct:crafting_station',
            {'id': '#forge:workbench', 'required': False}]})
        # The tag retains the station for other consumers. This recipe subtracts
        # it at matching, rather than globally removing it from the registry tag.
        predicate = json.loads((DATA / 'tconstruct/recipe/crafting_station.json').read_text())['key']['w']
        self.assertEqual(predicate['base'], '#forge:workbenches')
        self.assertEqual(predicate['subtracted'], 'tconstruct:crafting_station')
        self.assertEqual(set(predicate), {'type', 'base', 'subtracted'})
        # There is no separate fallback union that could survive a world tag replacement.
        self.assertIsInstance(predicate['base'], str)

    def test_existing_discovery_keeps_pattern_or_known_recipe(self):
        expected = json.loads((FIXTURES / 'advancement.json').read_text())
        expected['criteria']['has_the_recipe']['conditions'] = {'recipes': 'tconstruct:crafting_station'}
        expected['rewards']['recipes'] = ['tconstruct:crafting_station']
        native = json.loads((DATA / 'tconstruct/advancement/recipes/decorations/crafting_station.json').read_text())
        self.assertEqual(native, expected)
        self.assertEqual(native['requirements'], [['has_item', 'has_the_recipe']])
        self.assertEqual(native['criteria']['has_item']['conditions']['items'], [{'items': ['tconstruct:pattern']}])

    def test_package_contains_recipe_native_tag_and_existing_discovery(self):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / 'tconstruct.hmod'
            package(output)
            with ZipFile(output) as archive:
                for path in ('tconstruct/recipe/crafting_station.json', 'forge/tags/item/workbenches.json',
                    'tconstruct/advancement/recipes/decorations/crafting_station.json'):
                    self.assertEqual(archive.read('data/' + path), (DATA / path).read_bytes())


if __name__ == '__main__':
    unittest.main()
