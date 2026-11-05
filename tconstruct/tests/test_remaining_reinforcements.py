"""Pinned casting outputs must be registered and render in the native pack."""
import hashlib
import json
from pathlib import Path
import unittest
import sys
import tempfile
from zipfile import ZipFile

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / 'tools'))
from package_hmod import package

MATERIALS = {'iron': (90, 60), 'gold': (90, 57), 'cobalt': (90, 65), 'obsidian': (1000, 221)}

class ReinforcementAcquisition(unittest.TestCase):
    def test_pinned_recipes_and_native_assets(self):
        with ZipFile(ROOT / 'content/reference/tconstruct-recipes.zip') as archive:
            for material, (amount, ticks) in MATERIALS.items():
                with self.subTest(material=material):
                    name = material + '_reinforcement'
                    recipe = json.loads(archive.read(f'data/tconstruct/recipes/tools/modifiers/{name}.json'))
                    self.assertEqual(recipe, json.loads((ROOT / f'tests/fixtures/{name}.json').read_text()))
                    self.assertEqual(recipe['result'], 'tconstruct:' + name)
                    self.assertEqual(recipe['cast'], {'item': 'tconstruct:pattern'})
                    self.assertTrue(recipe['cast_consumed'])
                    self.assertEqual((recipe['fluid']['amount'], recipe['cooling_time']), (amount, ticks))
                    self.assertEqual(json.loads((ROOT / f'content/data/tconstruct/horizon/item/{name}.json').read_text()), {'template': 'minecraft:flint'})
                    definition = json.loads((ROOT / f'content/assets/tconstruct/items/{name}.json').read_text())
                    self.assertEqual(definition['model']['model'], 'tconstruct:item/' + name)
                    model = json.loads((ROOT / f'content/assets/tconstruct/models/item/{name}.json').read_text())
                    self.assertEqual(model, {'parent': 'minecraft:item/generated', 'textures': {'layer0': 'tconstruct:item/materials/reinforcement/' + material}})
                    texture = ROOT / f'content/assets/tconstruct/textures/item/materials/reinforcement/{material}.png'
                    hashes = json.loads((ROOT / 'tests/fixtures/reinforcement-textures.sha256.json').read_text())
                    self.assertEqual(hashlib.sha256(texture.read_bytes()).hexdigest(), hashes[material])

    def test_native_package_contains_outputs_and_source_recipes(self):
        with tempfile.TemporaryDirectory() as temp:
            output = Path(temp) / 'reinforcements.hmod'
            package(output)
            with ZipFile(output) as archive:
                for material in MATERIALS:
                    name = material + '_reinforcement'
                    for path in (f'data/tconstruct/horizon/item/{name}.json',
                                 f'assets/tconstruct/items/{name}.json',
                                 f'assets/tconstruct/models/item/{name}.json',
                                 f'assets/tconstruct/textures/item/materials/reinforcement/{material}.png'):
                        self.assertEqual(archive.read(path), (ROOT / 'content' / path).read_bytes())
                    recipe = json.loads(archive.read(f'assets/tconstruct/source-packs/tconstruct-original-pack/data/tconstruct/recipes/tools/modifiers/{name}.json'))
                    self.assertEqual(recipe, json.loads((ROOT / f'tests/fixtures/{name}.json').read_text()))
