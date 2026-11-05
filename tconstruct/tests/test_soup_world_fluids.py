"""Cool food liquids must not inherit molten-metal contact policies."""
import json
from pathlib import Path
import unittest

CONTENT = Path(__file__).resolve().parents[1] / 'content'

# Pinned TinkerFluids.cool and FluidTextureProvider.tintedStew.
SOUPS = [
    ('beetroot_soup', 0x84160D),
    ('mushroom_stew', 0xCD8C6F),
    ('rabbit_stew', 0x984A2C),
    ('meat_soup', 0xE03E35),
]


class SoupWorldFluidTests(unittest.TestCase):
    def test_cool_flow_and_player_contact_without_water_identity_aliases(self):
        for name, tint in SOUPS:
            with self.subTest(soup=name):
                fluid = json.loads((CONTENT / 'data/tconstruct/registered_fluid' / (name + '.json')).read_text())
                self.assertEqual(fluid['source'], 'tconstruct:' + name)
                self.assertEqual(fluid['flowing'], 'tconstruct:flowing_' + name)
                self.assertEqual(fluid['block'], 'tconstruct:' + name + '_fluid')
                self.assertEqual(fluid['bucket'], 'tconstruct:' + name + '_bucket')
                # register(), not registerSlime()/registerMetal(): Mantle defaults.
                self.assertEqual(fluid['policy'], {
                    'drop_off': 1, 'slope_distance': 4,
                    'tick_delay': 5, 'source_conversion': False,
                })
                policy = fluid['interaction']
                self.assertTrue(policy['can_swim'])
                self.assertTrue(policy['can_drown'])
                self.assertTrue(policy['extinguishes'])
                self.assertIsNone(policy['burn'])
                self.assertEqual(policy['motion_scale'], 0.0023333333333333335)
                self.assertEqual(policy['fall_distance_scale'], 0.5)
                self.assertEqual(fluid['visual'], {
                    'still': 'tconstruct:fluid/food/stew/still',
                    'flowing': 'tconstruct:fluid/food/stew/flowing',
                    'tint': tint, 'emission': 0,
                    'fog': {'start': 0.25, 'end': 2, 'color': tint},
                    'camera': {'texture': 'tconstruct:fluid/food/stew/camera', 'opacity': 0.1},
                })

    def test_nonitem_liquid_blocks_share_art_without_replacing_food_items(self):
        for name, _ in SOUPS:
            with self.subTest(soup=name):
                blockname = name + '_fluid'
                block = json.loads((CONTENT / 'data/tconstruct/horizon/block' / (blockname + '.json')).read_text())
                self.assertEqual(block['template'], 'minecraft:water')
                self.assertFalse(block['item'])
                self.assertEqual(block['properties']['light'], 0)
                self.assertFalse(block['properties']['random_ticks'])
                state = json.loads((CONTENT / 'assets/tconstruct/blockstates' / (blockname + '.json')).read_text())
                self.assertEqual(state['variants']['']['model'], 'tconstruct:block/' + blockname)
                model = json.loads((CONTENT / 'assets/tconstruct/models/block' / (blockname + '.json')).read_text())
                self.assertEqual(model['elements'], [])
                self.assertEqual(model['textures']['particle'], 'tconstruct:fluid/food/stew/still')
                self.assertFalse((CONTENT / 'assets/tconstruct/items' / (blockname + '.json')).exists())
                self.assertFalse((CONTENT / 'data/tconstruct/loot_table/blocks' / (blockname + '.json')).exists())


if __name__ == '__main__':
    unittest.main()
