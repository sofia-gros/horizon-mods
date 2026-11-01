"""Source geode data and placement inputs, without building a game binary."""
import json
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1] / 'content/data/tconstruct'

class SkyGeodeTest(unittest.TestCase):
    def test_source_configuration(self):
        feature = json.loads((ROOT / 'worldgen/configured_feature/sky_geode.json').read_text())
        self.assertEqual(feature['type'], 'minecraft:geode')
        self.assertEqual(feature['layers'], {'filling': 1.7, 'inner_layer': 2.2, 'middle_layer': 3.2, 'outer_layer': 4.2})
        self.assertEqual(feature['blocks']['outer_layer_provider']['id'], 'minecraft:mossy_cobblestone')
        self.assertEqual(feature['blocks']['inner_placements'], [
            'tconstruct:small_sky_slime_crystal_bud', 'tconstruct:medium_sky_slime_crystal_bud',
            'tconstruct:large_sky_slime_crystal_bud', 'tconstruct:sky_slime_crystal_cluster', 'tconstruct:steel_cluster'])
        self.assertEqual(feature['outer_wall_distance']['min_inclusive'], 4)
        self.assertEqual(feature['outer_wall_distance']['max_inclusive'], 7)
        placed = json.loads((ROOT / 'worldgen/placed_feature/sky_geode.json').read_text())
        self.assertEqual(placed['placement'][0]['chance'], 128)
        self.assertEqual(placed['placement'][2]['height']['min_inclusive'], {'above_bottom': 24})
        self.assertEqual(placed['placement'][2]['height']['max_inclusive'], {'absolute': 30})

    def test_cluster_drops_and_declared_blocks(self):
        for cluster, item in [('sky_slime_crystal_cluster', 'sky_slime_crystal'), ('steel_cluster', 'steel_shard')]:
            loot = (ROOT / 'loot_table/blocks' / (cluster + '.json')).read_text()
            self.assertIn('tconstruct:' + item, loot)
            self.assertIn('minecraft:tool/can_silk_touch', loot)
            self.assertIn('minecraft:fortune', loot)
            self.assertTrue((ROOT / 'horizon/block' / (cluster + '.json')).is_file())

if __name__ == '__main__':
    unittest.main()
