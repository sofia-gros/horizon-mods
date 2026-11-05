"""Source policies for the standard and custom player contact-effect liquids."""
import json
from pathlib import Path
import unittest

CONTENT = Path(__file__).resolve().parents[1] / 'content'


class EffectWorldFluidTests(unittest.TestCase):
    def test_only_declared_contact_effects_opt_out_of_generic_cures(self):
        protected = set()
        for path in (CONTENT / 'data/tconstruct/registered_fluid').glob('*.json'):
            fluid = json.loads(path.read_text())
            effect = fluid['interaction'].get('effect')
            if effect is not None:
                self.assertTrue(effect['no_curatives'])
                protected.add(path.stem)
        self.assertEqual(protected, {'venom', 'honey', 'liquid_soul', 'earth_slime', 'sky_slime'})

    def test_source_effect_flow_temperature_class_and_sprites(self):
        # TinkerFluids and FluidTextureProvider at f4e77d43, not generated values.
        cases = [
            ('venom', 'poison', 1, 4, 5, 0, True, 'slime/venom'),
            ('earth_slime', 'tconstruct:bouncy', 1, 4, 50, 0, True, 'slime/earth'),
            ('sky_slime', 'tconstruct:ricochet', 1, 4, 50, 0, True, 'slime/sky'),
            ('honey', 'slowness', 1, 4, 50, 0, True, 'food/honey'),
            ('liquid_soul', 'blindness', 8, 8, 30, 2, False, 'slime/soul'),
        ]
        for name, effect, drop, slope, delay, light, cool, folder in cases:
            with self.subTest(fluid=name):
                fluid = json.loads((CONTENT / 'data/tconstruct/registered_fluid' / (name + '.json')).read_text())
                blockname = name + '_fluid'
                self.assertEqual(fluid['source'], 'tconstruct:' + name)
                self.assertEqual(fluid['flowing'], 'tconstruct:flowing_' + name)
                self.assertEqual(fluid['block'], 'tconstruct:' + blockname)
                self.assertEqual(fluid['bucket'], 'tconstruct:' + name + '_bucket')
                self.assertEqual(fluid['policy'], {'drop_off': drop, 'slope_distance': slope,
                                                  'tick_delay': delay, 'source_conversion': False})
                interaction = fluid['interaction']
                self.assertEqual(interaction['effect'], {'id': effect if ':' in effect else 'minecraft:' + effect,
                                                        'duration_ticks': 100, 'amplifier': 0,
                                                        'no_curatives': True})
                self.assertIsNone(interaction['burn'])
                for key in ('can_swim', 'can_drown', 'extinguishes'):
                    self.assertEqual(interaction[key], cool)
                self.assertEqual(interaction['motion_scale'], 0.0023333333333333335)
                self.assertEqual(fluid['visual']['emission'], light)
                self.assertEqual(fluid['visual']['tint'], 0xFFFFFF)
                for field in ('still', 'flowing'):
                    self.assertEqual(fluid['visual'][field], 'tconstruct:fluid/' + folder + '/' + field)
                block = json.loads((CONTENT / 'data/tconstruct/horizon/block' / (blockname + '.json')).read_text())
                self.assertEqual(block['template'], 'minecraft:water')
                self.assertFalse(block['item'])
                self.assertEqual(block['properties']['light'], light)
                self.assertFalse(block['properties']['random_ticks'])
                state = json.loads((CONTENT / 'assets/tconstruct/blockstates' / (blockname + '.json')).read_text())
                self.assertEqual(state['variants']['']['model'], 'tconstruct:block/' + blockname)
                model = json.loads((CONTENT / 'assets/tconstruct/models/block' / (blockname + '.json')).read_text())
                self.assertEqual(model['elements'], [])
                self.assertEqual(model['textures']['particle'], fluid['visual']['still'])
                self.assertFalse((CONTENT / 'assets/tconstruct/items' / (blockname + '.json')).exists())
                self.assertFalse((CONTENT / 'data/tconstruct/loot_table/blocks' / (blockname + '.json')).exists())


if __name__ == '__main__':
    unittest.main()
