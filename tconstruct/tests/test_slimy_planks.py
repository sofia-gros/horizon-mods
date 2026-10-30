"""Validate the pinned source assets and shared slimewood block declarations."""
import hashlib
import json
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]

class SlimyPlanks(unittest.TestCase):
    def test_source_assets_and_block_properties(self):
        rows = (ROOT / 'docs/upstream-slimy-planks.sha256').read_text().splitlines()
        self.assertEqual(len(rows), 20)
        for row in rows:
            digest, name = row.split('  ', 1)
            self.assertEqual(hashlib.sha256((ROOT / name).read_bytes()).hexdigest(), digest, name)
        for name, color in [('greenheart', 19), ('skyroot', 23), ('bloodshroom', 28), ('enderbark', 29)]:
            short = name + '_planks'
            block = json.loads((ROOT / 'content/data/tconstruct/horizon/block' / (short + '.json')).read_text())
            self.assertEqual(block, {'template': 'minecraft:crimson_planks', 'properties': {
                'hardness': 2.0, 'explosion_resistance': 3.0, 'requires_tool': False,
                'sounds': 'minecraft:slime_block', 'map_color': color}})
            model = json.loads((ROOT / 'content/assets/tconstruct/items' / (short + '.json')).read_text())
            self.assertEqual(model['model']['model'], 'tconstruct:item/' + short)
            loot = json.loads((ROOT / 'content/data/tconstruct/loot_table/blocks' / (short + '.json')).read_text())
            self.assertEqual(loot['pools'][0]['condition']['type'], 'minecraft:survives_explosion')
            self.assertEqual(loot['pools'][0]['entries'][0]['name'], 'tconstruct:' + short)

if __name__ == '__main__':
    unittest.main()
