"""Verify bacon's source nutrition, native content, and package."""
import hashlib
import json
from pathlib import Path
import sys
import tempfile
import unittest
from zipfile import ZipFile

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / 'tools'))
from package_hmod import package

class BaconFood(unittest.TestCase):
    def test_source_and_native_food_values(self):
        raw = (ROOT / 'tests/fixtures/tinker_food_source.java').read_bytes()
        self.assertEqual(hashlib.sha1(f'blob {len(raw)}\0'.encode() + raw).hexdigest(), 'f3156aa8a0793b6e1d3de368e30311f6d3c46c57')
        source = raw.decode()
        self.assertIn('BACON = (new FoodProperties.Builder()).nutrition(4).saturationMod(0.6F).build()', source)
        native = json.loads((ROOT / 'content/data/tconstruct/horizon/item/bacon.json').read_text())
        self.assertEqual(native['template'], 'minecraft:cooked_porkchop')
        self.assertEqual(native['components']['minecraft:food'], {'nutrition': 4, 'saturation': 4.8})

    def test_package_has_food_model_texture_and_tab(self):
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp) / 'tconstruct.hmod'
            package(out)
            with ZipFile(out) as packed:
                item = json.loads(packed.read('data/tconstruct/horizon/item/bacon.json'))
                self.assertEqual(item['components']['minecraft:food'], {'nutrition': 4, 'saturation': 4.8})
                for name in ('assets/tconstruct/items/bacon.json', 'assets/tconstruct/models/item/bacon.json', 'assets/tconstruct/textures/item/food/bacon.png'):
                    self.assertEqual(packed.read(name), (ROOT / 'content' / name).read_bytes())
                names = [name for name in packed.namelist() if name.endswith('/creative-tabs.json')]
                self.assertEqual(len(names), 1)
                tabs = json.loads(packed.read(names[0]))
                general = next(tab for tab in tabs if tab['id'] == 'tconstruct:general')
                self.assertIn('{count:1,id:"tconstruct:bacon"}', general['items'])

if __name__ == '__main__':
    unittest.main()
