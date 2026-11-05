"""Source contracts for Ichor and Molten Cinderslime, deferred to final testing."""
import json
from pathlib import Path
import unittest
import zipfile

CONTENT = Path(__file__).resolve().parents[1] / 'content'


class InvertedWorldFluidTests(unittest.TestCase):
    def test_upward_families_keep_source_flow_and_contact_properties(self):
        # Pinned TinkerFluids and FluidDeferredRegisterExtension, not inferred
        # from density/temperature or copied from another liquid's declaration.
        for name, folder, drop, slope, delay, light, cool, burn in [
                ('ichor', 'slime/ichor', 1, 4, 50, 0, True, None),
                ('molten_cinderslime', 'molten/alloy/cinderslime', 3, 3, 30,
                 10, False, {'seconds': 10, 'damage': 7})]:
            with self.subTest(fluid=name):
                fluid = json.loads((CONTENT / 'data/tconstruct/registered_fluid' / (name + '.json')).read_text())
                self.assertEqual(fluid['source'], 'tconstruct:' + name)
                self.assertEqual(fluid['flowing'], 'tconstruct:flowing_' + name)
                self.assertEqual(fluid['block'], 'tconstruct:' + name + '_fluid')
                self.assertEqual(fluid['bucket'], 'tconstruct:' + name + '_bucket')
                self.assertEqual(fluid['policy'], {
                    'drop_off': drop, 'slope_distance': slope, 'tick_delay': delay,
                    'source_conversion': False, 'vertical_direction': 'up'})
                interaction = fluid['interaction']
                self.assertEqual(interaction['burn'], burn)
                for key in ('can_swim', 'can_drown', 'extinguishes'):
                    self.assertEqual(interaction[key], cool)
                self.assertEqual(interaction['motion_scale'], 0.0023333333333333335)
                self.assertEqual(interaction['fall_distance_scale'], 0.5)
                self.assertNotIn('effect', interaction)
                self.assertEqual(fluid['visual'], {
                    'still': 'tconstruct:fluid/' + folder + '/still',
                    'flowing': 'tconstruct:fluid/' + folder + '/flowing',
                    'tint': 0xFFFFFF, 'emission': light})
                blockname = name + '_fluid'
                block = json.loads((CONTENT / 'data/tconstruct/horizon/block' / (blockname + '.json')).read_text())
                self.assertEqual(block['template'], 'minecraft:water')
                self.assertFalse(block['item'])
                self.assertEqual(block['properties']['light'], light)
                self.assertFalse(block['properties']['random_ticks'])
                state = json.loads((CONTENT / 'assets/tconstruct/blockstates' / (blockname + '.json')).read_text())
                self.assertEqual(state['variants']['']['model'], 'tconstruct:block/' + blockname)
                model = json.loads((CONTENT / 'assets/tconstruct/models/block' / (blockname + '.json')).read_text())
                self.assertEqual(model, {'textures': {'particle': fluid['visual']['still']}, 'elements': []})
                self.assertFalse((CONTENT / 'assets/tconstruct/items' / (blockname + '.json')).exists())
                self.assertFalse((CONTENT / 'data/tconstruct/loot_table/blocks' / (blockname + '.json')).exists())
                with zipfile.ZipFile(CONTENT / 'reference/tconstruct-fluid-buckets.zip') as archive:
                    manifest = json.loads(archive.read('manifest.json'))
                self.assertEqual(manifest['buckets'][name + '_bucket']['fluid'], fluid['source'])

    def test_inverted_world_sprites_keep_source_reversed_frames(self):
        for folder, count, frametime, interpolate in [
                ('slime/ichor', 32, 3, True),
                ('molten/alloy/cinderslime', 39, 2, False)]:
            with self.subTest(sprite=folder):
                path = CONTENT / 'assets/tconstruct/textures/fluid' / folder
                animation = json.loads((path / 'flowing_inverted.png.mcmeta').read_text())['animation']
                self.assertEqual(animation['frames'], list(reversed(range(count))))
                self.assertEqual(animation['frametime'], frametime)
                self.assertEqual(animation.get('interpolate', False), interpolate)
                self.assertNotEqual((path / 'flowing.png').read_bytes(),
                                    (path / 'flowing_inverted.png').read_bytes())


if __name__ == '__main__':
    unittest.main()
