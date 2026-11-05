"""Bounded source Tool Forge routes retain native Anvil identity and costs."""
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
FIXTURES = ROOT / 'tests/fixtures/tool_forge_acquisition'
METALS = ('slimesteel', 'amethyst_bronze', 'rose_gold', 'pig_iron', 'cinderslime',
          'queens_slime', 'manyullyn', 'hepatizon', 'knightmetal', 'knightslime', 'netherite')
FAMILIES = {'tinkers_forge': 'tinkers_anvil', 'scorched_forge': 'scorched_anvil'}
sys.path.insert(0, str(ROOT / 'tools'))
from package_hmod import package


def variants(family):
    return [family + '_' + metal for metal in METALS] + ['zz_' + family + '_mixed_alloys']


class ToolForgeAcquisition(unittest.TestCase):
    def test_exact_source_recipe_and_discovery_fixtures(self):
        manifest = json.loads((FIXTURES / 'manifest.json').read_text())
        self.assertEqual(set(manifest), {folder + '/' + family + '.json'
            for folder in ('recipes', 'advancements') for family in FAMILIES})
        with ZipFile(ROOT / 'content/reference/tconstruct-recipes.zip') as source:
            for relative, entry in manifest.items():
                raw = (FIXTURES / relative).read_bytes()
                self.assertEqual(hashlib.sha256(raw).hexdigest(), entry['sha256'])
                self.assertIn('/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/', entry['source'])
                if relative.startswith('recipes/'):
                    self.assertEqual(json.loads(raw), json.loads(source.read(
                        'data/tconstruct/recipes/tables/' + Path(relative).name)))

    def test_twenty_four_routes_preserve_source_costs_and_existing_outputs(self):
        expected_ids = {name for family in FAMILIES for name in variants(family)}
        actual = {p.stem for p in (DATA / 'recipe').glob('*forge*.json')}
        self.assertEqual(actual, expected_ids)
        self.assertEqual(len(actual), 24)
        for forge, anvil in FAMILIES.items():
            source = json.loads((FIXTURES / 'recipes' / (forge + '.json')).read_text())
            for name in variants(forge):
                with self.subTest(recipe=name):
                    native = json.loads((DATA / 'recipe' / (name + '.json')).read_text())
                    base = json.loads((DATA / 'recipe' / (name.replace(forge, anvil) + '.json')).read_text())
                    self.assertEqual(native['type'], 'minecraft:crafting_shaped')
                    self.assertEqual(native['category'], source['category'])
                    self.assertEqual(native['pattern'], source['pattern'])
                    self.assertEqual(native['key'], {'s': '#' + source['key']['s']['tag'],
                        'm': base['key']['m'], 't': source['key']['t']['item']})
                    self.assertEqual(''.join(native['pattern']).count('s'), 3)
                    self.assertEqual(''.join(native['pattern']).count('m'), 4)
                    self.assertEqual(''.join(native['pattern']).count('t'), 1)
                    expected_result = copy.deepcopy(base['result'])
                    expected_result.setdefault('components', {})['minecraft:custom_name'] = json.loads(
                        source['result']['nbt']['display']['Name'])
                    self.assertEqual(native['result'], expected_result)
                    self.assertEqual(native['result']['id'], source['result']['item'])
                    self.assertEqual(native['result'].get('count', 1), 1)
                    self.assertTrue(native['show_notification'])
                    self.assertNotIn('tconstruct:fake_storage_block', json.dumps(native))
                    self.assertNotIn('RequiredMaterial', json.dumps(native))
                    if name.startswith('zz_'):
                        self.assertEqual(set(native['result']['components']), {'minecraft:custom_name'})
                        self.assertEqual(len(native['key']['m']), 11)
                    else:
                        model = native['result']['components']['minecraft:item_model'].split(':')[1]
                        self.assertTrue((ROOT / 'content/assets/tconstruct/items' / (model + '.json')).is_file())

    def test_source_acquisition_or_any_native_unlock_rewards_bounded_variants(self):
        for family in FAMILIES:
            source = json.loads((FIXTURES / 'advancements' / (family + '.json')).read_text())
            goal = json.loads((DATA / 'advancement/recipes/decorations' / (family + '.json')).read_text())
            self.assertEqual(goal['parent'], source['parent'])
            self.assertEqual(goal['sends_telemetry_event'], source['sends_telemetry_event'])
            self.assertEqual(goal['criteria']['has_item'], {'trigger': 'minecraft:inventory_changed',
                'conditions': {'items': [{'items': '#' + source['criteria']['has_item']['conditions']['items'][0]['tag']}]}})
            self.assertEqual(len(goal['requirements']), 1)
            self.assertEqual(set(goal['requirements'][0]), set(goal['criteria']))
            self.assertEqual(len(goal['criteria']), 13)
            recipes = {'tconstruct:' + name for name in variants(family)}
            self.assertEqual(set(goal['rewards']['recipes']), recipes)
            for key, criterion in goal['criteria'].items():
                if key != 'has_item':
                    self.assertEqual(criterion['trigger'], 'minecraft:recipe_unlocked')
                    self.assertIn(criterion['conditions']['recipes'], recipes)
        self.assertIn('"tconstruct:anvil_metal"', (ROOT / 'mixins/tconstruct/source-advancement-tag.rs').read_text())

    def test_package_contains_all_native_routes_and_discovery(self):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / 'tconstruct.hmod'
            package(output)
            with ZipFile(output) as archive:
                for family in FAMILIES:
                    paths = ['advancement/recipes/decorations/' + family + '.json']
                    paths.extend('recipe/' + name + '.json' for name in variants(family))
                    for path in paths:
                        self.assertEqual(archive.read('data/tconstruct/' + path), (DATA / path).read_bytes())


if __name__ == '__main__':
    unittest.main()
