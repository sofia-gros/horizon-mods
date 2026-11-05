"""Source material Tool Forge recipes stay bounded and require equal materials."""
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
FIXTURES = ROOT / 'tests/fixtures/material_tool_forge_acquisition'
METALS = ('bronze', 'constantan', 'invar', 'electrum', 'pewter', 'nicrosil')
FAMILIES = [('seared_forge_material', 'tinkers_forge', 'tinkers_anvil'),
            ('scorched_forge_material', 'scorched_forge', 'scorched_anvil')]
sys.path.insert(0, str(ROOT / 'tools'))
from package_hmod import package


class MaterialToolForgeAcquisition(unittest.TestCase):
    def test_exact_pinned_sources_and_existing_compatibility_alloys(self):
        manifest = json.loads((FIXTURES / 'manifest.json').read_text())
        self.assertEqual(len(manifest), 4)
        for relative, entry in manifest.items():
            raw = (FIXTURES / relative).read_bytes()
            self.assertEqual(hashlib.sha256(raw).hexdigest(), entry['sha256'])
            self.assertIn('/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/', entry['source'])
        with ZipFile(ROOT / 'content/reference/tconstruct-materials.zip') as archive:
            source = json.loads(archive.read('data/tconstruct/tinkering/tags/materials/compatibility_metals/alloys.json'))
        values = {entry['id'] if isinstance(entry, dict) else entry for entry in source['values']}
        self.assertTrue({'tconstruct:' + metal for metal in METALS} <= values)

    def test_twelve_routes_reuse_material_guard_and_outputs_with_four_block_cost(self):
        expected = {forge + '_compat_' + metal for _, forge, _ in FAMILIES for metal in METALS}
        self.assertEqual({p.stem for p in (DATA / 'recipe').glob('*forge_compat_*.json')}, expected)
        for source_id, forge, anvil in FAMILIES:
            source = json.loads((FIXTURES / 'recipes' / (source_id + '.json')).read_text())
            self.assertEqual(source['key']['m']['material']['tag'], 'tconstruct:compatibility_metals/alloys')
            for metal in METALS:
                recipe = json.loads((DATA / 'recipe' / f'{forge}_compat_{metal}.json').read_text())
                base = json.loads((DATA / 'recipe' / f'{anvil}_compat_{metal}.json').read_text())
                self.assertEqual(recipe['type'], 'minecraft:crafting_shaped')
                self.assertEqual(recipe['pattern'], source['pattern'])
                self.assertEqual(''.join(recipe['pattern']).count('m'), 4)
                self.assertEqual(''.join(recipe['pattern']).count('s'), 3)
                self.assertEqual(recipe['key'], {'m': source['key']['m']['item'],
                    's': '#' + source['key']['s']['tag'], 't': source['key']['t']['item']})
                result = copy.deepcopy(base['result'])
                result['components']['minecraft:custom_name'] = json.loads(source['result']['nbt']['display']['Name'])
                self.assertEqual(recipe['result'], result)
                self.assertEqual(result['components']['minecraft:custom_data']['tconstruct:anvil']['RequiredMaterial'],
                                 'tconstruct:' + metal)
        guard = (ROOT / 'mixins/tconstruct/anvil-material-recipe.rs').read_text()
        self.assertIn('if is_forge { 4 } else { 3 }', guard)
        self.assertIn('[3, 5, 6, 8]', guard)
        self.assertIn('fake.len() != required_fake', guard)

    def test_source_fake_block_or_recipe_unlock_conditions_remain_one_or_group(self):
        for source_id, forge, _ in FAMILIES:
            source = json.loads((FIXTURES / 'advancements' / (source_id + '.json')).read_text())
            goal = json.loads((DATA / 'advancement/recipes/decorations' / (source_id + '.json')).read_text())
            self.assertEqual(goal['criteria']['has_item'], source['criteria']['has_item'])
            self.assertEqual(goal['parent'], source['parent'])
            self.assertEqual(goal['sends_telemetry_event'], source['sends_telemetry_event'])
            self.assertEqual(len(goal['criteria']), 7)
            self.assertEqual(len(goal['requirements']), 1)
            self.assertEqual(set(goal['requirements'][0]), set(goal['criteria']))
            recipes = {'tconstruct:' + forge + '_compat_' + metal for metal in METALS}
            self.assertEqual(set(goal['rewards']['recipes']), recipes)
            for key, condition in goal['criteria'].items():
                if key != 'has_item':
                    self.assertEqual(condition['trigger'], 'minecraft:recipe_unlocked')
                    self.assertIn(condition['conditions']['recipes'], recipes)

    def test_packaged_recipes_and_discovery(self):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / 'tconstruct.hmod'
            package(output)
            with ZipFile(output) as archive:
                for source_id, forge, _ in FAMILIES:
                    paths = ['advancement/recipes/decorations/' + source_id + '.json']
                    paths.extend('recipe/' + forge + '_compat_' + metal + '.json' for metal in METALS)
                    for path in paths:
                        self.assertEqual(archive.read('data/tconstruct/' + path), (DATA / path).read_bytes())


if __name__ == '__main__':
    unittest.main()
