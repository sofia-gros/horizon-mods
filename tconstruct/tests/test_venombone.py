"""Venombone's registered casting output and material input match pinned data."""
import hashlib
import json
from pathlib import Path
import sys
import tempfile
import unittest
import zipfile

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / 'tools'))
from package_hmod import package

TEXTURE_HASH = '3dbbd011d9d046d4a62069812d8d148710e283a6249810b810d107aa71164625'


class Venombone(unittest.TestCase):
    def test_pinned_casting_and_material_inputs(self):
        with zipfile.ZipFile(ROOT / 'content/reference/tconstruct-recipes.zip') as archive:
            casting = json.loads(archive.read('data/tconstruct/recipes/smeltery/casting/slime/venom/bone.json'))
            material = json.loads(archive.read('data/tconstruct/recipes/tools/materials/venombone.json'))
        self.assertEqual(casting, {
            'type': 'tconstruct:casting_table', 'cast': {'tag': 'forge:bones'},
            'cast_consumed': True, 'cooling_time': 48,
            'fluid': {'amount': 250, 'tag': 'tconstruct:venom'},
            'result': 'tconstruct:venombone',
        })
        self.assertEqual(material, {
            'type': 'tconstruct:material', 'ingredient': {'item': casting['result']},
            'material': 'tconstruct:venombone', 'needed': 1, 'value': 1,
        })
        self.assertEqual(material, json.loads((ROOT / 'content/data/tconstruct/recipes/tools/materials/venombone.json').read_text()))
        with zipfile.ZipFile(ROOT / 'content/reference/tconstruct-materials.zip') as archive:
            definition = json.loads(archive.read('data/tconstruct/tinkering/materials/definition/venombone.json'))
        self.assertTrue(definition['craftable'])

    def test_native_package_has_registered_output_and_source_visuals(self):
        with tempfile.TemporaryDirectory() as tmp:
            output = Path(tmp) / 'test.hmod'
            package(output)
            with zipfile.ZipFile(output) as archive:
                self.assertEqual(json.loads(archive.read('data/tconstruct/horizon/item/venombone.json')), {'template': 'minecraft:bone'})
                self.assertEqual(json.loads(archive.read('assets/tconstruct/items/venombone.json')), {'model': {'type': 'minecraft:model', 'model': 'tconstruct:item/venombone'}})
                self.assertEqual(json.loads(archive.read('assets/tconstruct/models/item/venombone.json')), {'parent': 'minecraft:item/bone', 'textures': {'layer0': 'tconstruct:item/materials/venombone'}})
                self.assertEqual(hashlib.sha256(archive.read('assets/tconstruct/textures/item/materials/venombone.png')).hexdigest(), TEXTURE_HASH)
                self.assertIn('assets/tconstruct/licenses/tconstruct.txt', archive.namelist())


if __name__ == '__main__':
    unittest.main()
