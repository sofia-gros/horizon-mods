"""Pinned content contracts for the remaining conventional molten metals."""
import json
from pathlib import Path
import unittest

CONTENT = Path(__file__).resolve().parents[1] / 'content'

# TConstruct f4e77d43 TinkerFluids and FluidTextureProvider:
# short name, burn damage, block light, fluid emission, source texture, RGB tint.
CASES = [
    ('steel', 6, 13, 13, 'ore/steel', 0xFFFFFF),
    ('knightmetal', 8, 10, 10, 'alloy/knightmetal', 0xFFFFFF),
    ('knightslime', 8, 12, 12, 'alloy/knightslime', 0xFFFFFF),
    ('tin', 5, 12, 12, 'compat_ore/tin', 0xFFFFFF),
    ('aluminum', 5, 12, 12, 'compat_ore/aluminum', 0xFFFFFF),
    ('lead', 5, 12, 12, 'compat_ore/lead', 0xFFFFFF),
    ('silver', 5, 12, 12, 'compat_ore/silver', 0xFFFFFF),
    ('nickel', 5, 12, 12, 'compat_ore/nickel', 0xFFFFFF),
    ('zinc', 5, 12, 12, 'compat_ore/zinc', 0xFFFFFF),
    ('platinum', 5, 12, 12, 'compat_ore/platinum', 0xFFFFFF),
    ('tungsten', 5, 12, 12, 'compat_ore/tungsten', 0xFFFFFF),
    ('osmium', 5, 4, 4, 'compat_ore/osmium', 0xFFFFFF),
    ('uranium', 5, 15, 15, 'compat_ore/uranium', 0xFFFFFF),
    ('chromium', 5, 13, 13, '', 0xC4B180),
    ('cadmium', 5, 10, 10, '', 0x8AB8C8),
    ('bronze', 6, 10, 10, 'compat_alloy/bronze', 0xFFFFFF),
    ('brass', 6, 10, 10, 'compat_alloy/brass', 0xFFFFFF),
    ('electrum', 6, 10, 10, 'compat_alloy/electrum', 0xFFFFFF),
    ('invar', 6, 10, 10, 'compat_alloy/invar', 0xFFFFFF),
    ('constantan', 6, 10, 10, 'compat_alloy/constantan', 0xFFFFFF),
    ('pewter', 6, 10, 10, 'compat_alloy/pewter', 0xFFFFFF),
    ('enderium', 7, 12, 12, 'compat_alloy/enderium', 0xFFFFFF),
    ('lumium', 7, 15, 15, 'compat_alloy/lumium', 0xFFFFFF),
    ('signalum', 7, 13, 13, 'compat_alloy/signalum', 0xFFFFFF),
    ('refined_glowstone', 7, 15, 15, 'compat_alloy/refined_glowstone', 0xFFFFFF),
    ('refined_obsidian', 7, 7, 7, 'compat_alloy/refined_obsidian', 0xFFFFFF),
    ('nicrosil', 6, 12, 14, 'compat_alloy/nicrosil', 0xFFFFFF),
    ('duralumin', 6, 10, 10, 'compat_alloy/duralumin', 0xFFFFFF),
    ('bendalloy', 6, 9, 9, '', 0xD6D2C1),
    ('steeleaf', 6, 10, 10, 'compat_ore/steeleaf', 0xFFFFFF),
]


class RemainingWorldMetalTests(unittest.TestCase):
    def test_source_policies_and_separate_liquid_block_assets(self):
        self.assertEqual(len(CASES), 30)
        for name, damage, light, emission, texture, tint in CASES:
            with self.subTest(metal=name):
                short = 'molten_' + name
                blockname = short + '_fluid'
                fluid = json.loads((CONTENT / 'data/tconstruct/registered_fluid' / (short + '.json')).read_text())
                self.assertEqual(fluid['source'], 'tconstruct:' + short)
                self.assertEqual(fluid['flowing'], 'tconstruct:flowing_' + short)
                self.assertEqual(fluid['block'], 'tconstruct:' + blockname)
                self.assertEqual(fluid['bucket'], 'tconstruct:' + short + '_bucket')
                self.assertEqual(fluid['policy'], {'drop_off': 3, 'slope_distance': 3,
                                                  'tick_delay': 30, 'source_conversion': False})
                self.assertEqual(fluid['interaction']['burn'], {'seconds': 10, 'damage': damage})
                for field in ('can_swim', 'can_drown', 'extinguishes'):
                    self.assertFalse(fluid['interaction'][field])
                self.assertEqual(fluid['interaction']['motion_scale'], 0.0023333333333333335)
                block = json.loads((CONTENT / 'data/tconstruct/horizon/block' / (blockname + '.json')).read_text())
                self.assertEqual(block['template'], 'minecraft:water')
                self.assertFalse(block['item'])
                self.assertEqual(block['properties']['light'], light)
                self.assertEqual(fluid['visual']['emission'], emission)
                self.assertEqual(fluid['visual']['tint'], tint)
                prefix = 'tconstruct:fluid/molten/' + (texture + '/' if texture else '')
                for field in ('still', 'flowing'):
                    self.assertEqual(fluid['visual'][field], prefix + field)
                state = json.loads((CONTENT / 'assets/tconstruct/blockstates' / (blockname + '.json')).read_text())
                self.assertEqual(state['variants']['']['model'], 'tconstruct:block/' + blockname)
                model = json.loads((CONTENT / 'assets/tconstruct/models/block' / (blockname + '.json')).read_text())
                self.assertEqual(model, {'textures': {'particle': prefix + 'still'}, 'elements': []})
                self.assertFalse((CONTENT / 'assets/tconstruct/items' / (blockname + '.json')).exists())
                self.assertFalse((CONTENT / 'data/tconstruct/loot_table/blocks' / (blockname + '.json')).exists())

    def test_nicrosil_retains_distinct_source_fluid_and_block_light(self):
        fluid = json.loads((CONTENT / 'data/tconstruct/registered_fluid/molten_nicrosil.json').read_text())
        block = json.loads((CONTENT / 'data/tconstruct/horizon/block/molten_nicrosil_fluid.json').read_text())
        self.assertEqual(fluid['visual']['emission'], 14)
        self.assertEqual(block['properties']['light'], 12)


if __name__ == '__main__':
    unittest.main()
