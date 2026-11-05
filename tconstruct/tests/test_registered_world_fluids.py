"""Observable content and packaging contracts for registered world liquids."""
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
                ('netherite', 10, 14, 3, 30, 0xFFFFFF),
                ('slimesteel', 6, 10, 3, 30, 0xFFFFFF),
                ('amethyst_bronze', 6, 12, 3, 30, 0xFFFFFF),
                ('rose_gold', 6, 12, 3, 30, 0xFFFFFF),
                ('pig_iron', 6, 10, 3, 30, 0xFFFFFF),
                ('manyullyn', 8, 11, 3, 30, 0xFFFFFF),
                ('hepatizon', 7, 8, 3, 30, 0xFFFFFF),
                ('queens_slime', 6, 9, 3, 30, 0xFFFFFF),
                ('soulsteel', 7, 6, 3, 30, 0xFFFFFF),
                ('seared_stone', 2, 6, 2, 40, 0x4F4A47),
                ('scorched_stone', 2, 4, 2, 40, 0x3E3029),
                ('clay', 2, 3, 2, 40, 0x9B6045),
                ('porcelain', 2, 2, 2, 40, 0xFFFFFF),
                ('obsidian', 4, 3, 2, 40, 0xFFFFFF),
                ('emerald', 6, 9, 4, 45, 0xFFFFFF),
                ('quartz', 5, 6, 4, 45, 0xFFFFFF),
                ('amethyst', 5, 11, 4, 45, 0xFFFFFF),
                ('diamond', 7, 13, 4, 45, 0xFFFFFF),
                ('blazing_blood', 5, 15, 1, 5, 0xFFFFFF),
                ('fiery_liquid', 6, 15, 1, 30, 0xFFFFFF)]:
            with self.subTest(metal=name):
                short = name if name in ('seared_stone', 'scorched_stone', 'blazing_blood', 'fiery_liquid') else 'molten_' + name
                blockname = short + '_fluid'
                fluid = json.loads((CONTENT / 'data/tconstruct/registered_fluid' / (short + '.json')).read_text())
                self.assertEqual(fluid['source'], 'tconstruct:' + short)
                self.assertEqual(fluid['flowing'], 'tconstruct:flowing_' + short)
                self.assertEqual(fluid['block'], 'tconstruct:' + blockname)
                self.assertEqual(fluid['bucket'], 'tconstruct:' + short + '_bucket')
                slope = 4 if name in ('blazing_blood', 'fiery_liquid') else drop
                self.assertEqual(fluid['policy'], {'drop_off': drop, 'slope_distance': slope, 'tick_delay': delay, 'source_conversion': False})
                policy = fluid['interaction']
                seconds = {'seared_stone': 8, 'scorched_stone': 7,
                           'clay': 5, 'porcelain': 5, 'obsidian': 12,
                           'blazing_blood': 15, 'fiery_liquid': 20}.get(name, 10)
                self.assertEqual(policy['burn'], {'seconds': seconds, 'damage': damage})
                self.assertFalse(policy['can_swim'])
                self.assertFalse(policy['can_drown'])
                self.assertFalse(policy['extinguishes'])
                self.assertEqual(policy['motion_scale'], 0.0023333333333333335)
                block = json.loads((CONTENT / 'data/tconstruct/horizon/block' / (blockname + '.json')).read_text())
                self.assertEqual(block['template'], 'minecraft:water')
                self.assertFalse(block['item'])
                self.assertEqual(block['properties']['light'], light)
                self.assertEqual(fluid['visual']['emission'], light)
                self.assertEqual(fluid['visual']['tint'], tint)
                state = json.loads((CONTENT / 'assets/tconstruct/blockstates' / (blockname + '.json')).read_text())
                self.assertEqual(state['variants']['']['model'], 'tconstruct:block/' + blockname)
                model = json.loads((CONTENT / 'assets/tconstruct/models/block' / (blockname + '.json')).read_text())
                self.assertEqual(model['elements'], [])
                self.assertEqual(model['textures']['particle'], fluid['visual']['still'])
                self.assertFalse((CONTENT / 'assets/tconstruct/items' / (blockname + '.json')).exists())
                self.assertFalse((CONTENT / 'data/tconstruct/loot_table/blocks' / (blockname + '.json')).exists())

    def test_source_assets_keep_pixels_and_animation_metadata(self):
        manifest = json.loads((CONTENT / 'reference/registered-world-fluid-assets.json').read_text())
        self.assertEqual(manifest['revision'], 'f4e77d43f39ba1047bf93ae816b7e1beaab514c2')
        self.assertEqual(len(manifest['files']), 204)
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
                    families = ['molten_' + name for name in (
                        'iron', 'gold', 'copper', 'cobalt', 'debris', 'netherite',
                        'slimesteel', 'amethyst_bronze', 'rose_gold', 'pig_iron',
                        'manyullyn', 'hepatizon', 'queens_slime', 'soulsteel',
                        'clay', 'porcelain', 'obsidian', 'emerald', 'quartz', 'amethyst', 'diamond',
                        'steel', 'knightmetal', 'knightslime', 'tin', 'aluminum', 'lead', 'silver',
                        'nickel', 'zinc', 'platinum', 'tungsten', 'osmium', 'uranium', 'chromium',
                        'cadmium', 'bronze', 'brass', 'electrum', 'invar', 'constantan', 'pewter',
                        'enderium', 'lumium', 'signalum', 'refined_glowstone', 'refined_obsidian',
                        'nicrosil', 'duralumin', 'bendalloy', 'steeleaf')]
                    for short in families + ['seared_stone', 'scorched_stone', 'blazing_blood', 'fiery_liquid',
                                             'beetroot_soup', 'mushroom_stew', 'rabbit_stew', 'meat_soup']:
                        definition = json.loads(archive.read('data/tconstruct/registered_fluid/' + short + '.json'))
                        blockname = definition['block'].split(':', 1)[1]
                        archive.read('data/tconstruct/horizon/block/' + blockname + '.json')
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

    def test_overworld_alloys_use_distinct_source_sprite_pairs(self):
        pairs = set()
        for name in ('slimesteel', 'amethyst_bronze', 'rose_gold', 'pig_iron'):
            with self.subTest(fluid=name):
                definition = json.loads((CONTENT / 'data/tconstruct/registered_fluid' / ('molten_' + name + '.json')).read_text())
                for field in ('still', 'flowing'):
                    self.assertEqual(definition['visual'][field],
                                     'tconstruct:fluid/molten/alloy/' + name + '/' + field)
                pairs.add((definition['visual']['still'], definition['visual']['flowing']))
        self.assertEqual(len(pairs), 4)

    def test_advanced_alloys_use_dedicated_sprites_without_invented_inverted_flow(self):
        for name in ('manyullyn', 'hepatizon', 'queens_slime', 'soulsteel'):
            with self.subTest(fluid=name):
                definition = json.loads((CONTENT / 'data/tconstruct/registered_fluid' / ('molten_' + name + '.json')).read_text())
                for field in ('still', 'flowing'):
                    self.assertEqual(definition['visual'][field],
                                     'tconstruct:fluid/molten/alloy/' + name + '/' + field)
        # Cinderslime is a separate source InvertedFluid, not a downward alloy.
        self.assertFalse((CONTENT / 'data/tconstruct/registered_fluid/molten_cinderslime.json').exists())

    def test_stone_fluids_keep_solid_blocks_and_source_textures_separate(self):
        for name, template in [('seared_stone', 'minecraft:stone'),
                               ('scorched_stone', 'minecraft:basalt')]:
            with self.subTest(solid=name):
                block = json.loads((CONTENT / 'data/tconstruct/horizon/block' / (name + '.json')).read_text())
                self.assertEqual(block['template'], template)
                self.assertTrue(block.get('item', True))
                self.assertTrue((CONTENT / 'assets/tconstruct/items' / (name + '.json')).is_file())
                self.assertTrue((CONTENT / 'data/tconstruct/loot_table/blocks' / (name + '.json')).is_file())
        for name, folder in [('seared_stone', 'stone'), ('scorched_stone', 'stone'),
                             ('molten_clay', 'stone'), ('molten_porcelain', 'stone/porcelain'),
                             ('molten_obsidian', 'stone/obsidian')]:
            with self.subTest(fluid=name):
                fluid = json.loads((CONTENT / 'data/tconstruct/registered_fluid' / (name + '.json')).read_text())
                for field in ('still', 'flowing'):
                    self.assertEqual(fluid['visual'][field],
                                     'tconstruct:fluid/molten/' + folder + '/' + field)

    def test_gem_liquids_keep_source_gem_sprites_and_no_native_lava_alias(self):
        for gem in ('emerald', 'quartz', 'amethyst', 'diamond'):
            with self.subTest(gem=gem):
                fluid = json.loads((CONTENT / 'data/tconstruct/registered_fluid' / ('molten_' + gem + '.json')).read_text())
                self.assertEqual(fluid['block'], 'tconstruct:molten_' + gem + '_fluid')
                self.assertFalse(fluid['policy']['source_conversion'])
                for field in ('still', 'flowing'):
                    self.assertEqual(fluid['visual'][field],
                                     'tconstruct:fluid/molten/ore/' + gem + '/' + field)

    def test_burning_liquids_keep_distinct_default_flow_timing_and_source_sprites(self):
        for name, folder, delay in [('blazing_blood', 'molten/blaze', 5),
                                    ('fiery_liquid', 'slime/fiery', 30)]:
            with self.subTest(fluid=name):
                fluid = json.loads((CONTENT / 'data/tconstruct/registered_fluid' / (name + '.json')).read_text())
                # Mantle FluidBuilder defaults: level decrease 1, slope 4, tick 5.
                # Fiery Liquid alone overrides the tick delay to 30.
                self.assertEqual(fluid['policy'], {'drop_off': 1, 'slope_distance': 4,
                                                  'tick_delay': delay, 'source_conversion': False})
                for field in ('still', 'flowing'):
                    self.assertEqual(fluid['visual'][field],
                                     'tconstruct:fluid/' + folder + '/' + field)


if __name__ == '__main__':
    unittest.main()
