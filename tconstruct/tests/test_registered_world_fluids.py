"""Observable content contracts for the initial six world-liquid families."""
import hashlib
import importlib.util
import json
from pathlib import Path
import struct
import sys
import tempfile
import unittest
import zipfile

ROOT = Path(__file__).resolve().parents[1]
CONTENT = ROOT / 'content'


class RegisteredWorldFluidTests(unittest.TestCase):
    def test_source_flow_burn_and_native_water_separation(self):
        # Independent expectations from pinned TinkerFluids, registration
        # helpers and FluidTextureProvider. Debris uses gem flow, not metal.
        for name, damage, light, drop, delay, tint in [
                ('iron', 5, 12, 3, 30, 0xFFFFFF),
                ('gold', 5, 12, 3, 30, 0xFFFFFF),
                ('copper', 5, 12, 3, 30, 0xFFFFFF),
                ('cobalt', 6, 8, 3, 30, 0xFFFFFF),
                ('debris', 8, 14, 4, 45, 0x411E15),
                ('netherite', 10, 14, 3, 30, 0xFFFFFF)]:
            with self.subTest(metal=name):
                short = 'molten_' + name
                fluid = json.loads((CONTENT / 'data/tconstruct/registered_fluid' / (short + '.json')).read_text())
                self.assertEqual(fluid['source'], 'tconstruct:' + short)
                self.assertEqual(fluid['flowing'], 'tconstruct:flowing_' + short)
                self.assertEqual(fluid['block'], 'tconstruct:' + short)
                self.assertEqual(fluid['bucket'], 'tconstruct:' + short + '_bucket')
                self.assertEqual(fluid['policy'], {'drop_off': drop, 'slope_distance': drop, 'tick_delay': delay, 'source_conversion': False})
                policy = fluid['interaction']
                self.assertEqual(policy['burn'], {'seconds': 10, 'damage': damage})
                self.assertFalse(policy['can_swim'])
                self.assertFalse(policy['can_drown'])
                self.assertFalse(policy['extinguishes'])
                self.assertEqual(policy['motion_scale'], 0.0023333333333333335)
                block = json.loads((CONTENT / 'data/tconstruct/horizon/block' / (short + '.json')).read_text())
                self.assertEqual(block['template'], 'minecraft:water')
                self.assertFalse(block['item'])
                self.assertEqual(block['properties']['light'], light)
                self.assertEqual(fluid['visual']['emission'], light)
                self.assertEqual(fluid['visual']['tint'], tint)
                state = json.loads((CONTENT / 'assets/tconstruct/blockstates' / (short + '.json')).read_text())
                self.assertEqual(state['variants']['']['model'], 'tconstruct:block/' + short)
                model = json.loads((CONTENT / 'assets/tconstruct/models/block' / (short + '.json')).read_text())
                self.assertEqual(model['elements'], [])
                self.assertEqual(model['textures']['particle'], fluid['visual']['still'])
                self.assertFalse((CONTENT / 'assets/tconstruct/items' / (short + '.json')).exists())
                self.assertFalse((CONTENT / 'data/tconstruct/loot_table/blocks' / (short + '.json')).exists())

    def test_source_assets_keep_pixels_and_animation_metadata(self):
        manifest = json.loads((CONTENT / 'reference/registered-world-fluid-assets.json').read_text())
        self.assertEqual(manifest['revision'], 'f4e77d43f39ba1047bf93ae816b7e1beaab514c2')
        self.assertEqual(len(manifest['files']), 24)
        for path, source in manifest['files'].items():
            with self.subTest(asset=path):
                data = (CONTENT / path).read_bytes()
                self.assertEqual(hashlib.sha256(data).hexdigest(), source['sha256'])
                if path.endswith('.png'):
                    self.assertEqual(data[:8], b'\x89PNG\r\n\x1a\n')
                    width, height = struct.unpack('>II', data[16:24])
                    self.assertIn(width, (16, 32))
                    self.assertEqual(height % width, 0)
                else:
                    self.assertIn('animation', json.loads(data))

    def test_packaged_content_contains_declarations_and_distinct_sprites(self):
        sys.path.insert(0, str(ROOT / 'tools'))
        try:
            spec = importlib.util.spec_from_file_location('world_fluid_package', ROOT / 'tools/package_hmod.py')
            module = importlib.util.module_from_spec(spec)
            spec.loader.exec_module(module)
            with tempfile.TemporaryDirectory() as tmp:
                target = Path(tmp) / 'tconstruct.hmod'
                module.package(target)
                with zipfile.ZipFile(target) as archive:
                    for metal in ('iron', 'gold', 'copper', 'cobalt', 'debris', 'netherite'):
                        definition = json.loads(archive.read('data/tconstruct/registered_fluid/molten_' + metal + '.json'))
                        bucket = definition['bucket'].split(':', 1)[1]
                        archive.read('data/tconstruct/horizon/item/' + bucket + '.json')
                        for field in ('still', 'flowing'):
                            sprite = definition['visual'][field].replace(':', '/textures/', 1)
                            archive.read('assets/' + sprite + '.png')
                            archive.read('assets/' + sprite + '.png.mcmeta')
        finally:
            sys.path.pop(0)

    def test_debris_uses_tinted_stone_and_netherite_uses_its_alloy_sprites(self):
        for name, folder in [('debris', 'stone'), ('netherite', 'alloy/netherite')]:
            with self.subTest(fluid=name):
                definition = json.loads((CONTENT / 'data/tconstruct/registered_fluid' / ('molten_' + name + '.json')).read_text())
                for field in ('still', 'flowing'):
                    self.assertEqual(definition['visual'][field],
                                     'tconstruct:fluid/molten/' + folder + '/' + field)


if __name__ == '__main__':
    unittest.main()
