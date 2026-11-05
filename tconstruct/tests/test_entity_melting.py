"""Pinned resources and packaging for the first-Blazing-Blood path."""
import hashlib
import json
from pathlib import Path
import unittest
import zipfile

ROOT = Path(__file__).resolve().parents[1]

class EntityMelting(unittest.TestCase):
    def test_source_archive_matches_pinned_manifest(self):
        expected = json.loads((ROOT / 'docs/upstream-entity-melting.sha256.json').read_text())
        with zipfile.ZipFile(ROOT / 'content/reference/tconstruct-entity-melting.zip') as archive:
            names = archive.namelist()
            self.assertEqual(len(names), len(set(names)))
            self.assertEqual(len(names), 103)
            actual = {name: hashlib.sha256(archive.read(name)).hexdigest() for name in names}
            self.assertEqual(actual, expected)
            self.assertEqual(json.loads(archive.read('data/forge/tags/entity_types/blazes.json'))['values'], ['minecraft:blaze'])
            self.assertEqual(json.loads(archive.read('data/tconstruct/tags/entity_types/melting/blacklist.json'))['values'], [])
            self.assertIn('tconstruct:smeltery_heat', json.loads(archive.read('data/minecraft/tags/damage_type/is_fire.json'))['values'])
            self.assertIn('tconstruct:smeltery_magic', json.loads(archive.read('data/minecraft/tags/damage_type/witch_resistant_to.json'))['values'])
            bypass = json.loads(archive.read('data/minecraft/tags/damage_type/bypasses_armor.json'))['values']
            self.assertNotIn('tconstruct:smeltery_magic', bypass)
            self.assertNotIn('tconstruct:smeltery_heat', bypass)

    def test_original_blaze_recipe_and_initial_lava_fuel(self):
        with zipfile.ZipFile(ROOT / 'content/reference/tconstruct-recipes.zip') as archive:
            blaze = json.loads(archive.read('data/tconstruct/recipes/smeltery/entity_melting/blaze.json'))
            self.assertEqual(blaze, {'type': 'tconstruct:entity_melting', 'damage': 2,
                'entity': {'tag': 'forge:blazes'}, 'result': {'amount': 20, 'fluid': 'tconstruct:blazing_blood'}})
            fuels = [json.loads(archive.read(name)) for name in archive.namelist() if name.endswith('.json') and '/fuel/' in name]
            self.assertTrue(any(value.get('type') == 'tconstruct:melting_fuel' and
                value.get('fluid', {}).get('fluid') == 'minecraft:lava' and value.get('temperature', 0) > 0 for value in fuels))

    def test_both_preparation_paths_retain_entity_sources(self):
        manual = (ROOT / 'mixins/tconstruct/prepare.rs').read_text()
        package = (ROOT / 'tools/package_hmod.py').read_text()
        self.assertIn('reference/tconstruct-entity-melting.zip', manual)
        self.assertIn("'tconstruct-entity-melting.zip': 'tconstruct-original-pack'", package)
        self.assertIn('entity-v1', (ROOT / 'mixins/tconstruct/resource-tables.rs').read_text())
