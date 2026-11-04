"""Pin the independent Scorched Basin subset and its explicit acquisition boundary."""
import json
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CONTENT = ROOT / 'content'
MIXINS = ROOT / 'mixins/tconstruct'

class ScorchedBasinTests(unittest.TestCase):
    def test_block_and_item_assets(self):
        def load(path):
            return json.loads((CONTENT / path).read_text())
        block = load('data/tconstruct/horizon/block/scorched_basin.json')
        self.assertEqual(block['template'], 'minecraft:white_glazed_terracotta')
        self.assertEqual(block['properties'], {'hardness': 2.5, 'explosion_resistance': 8.0,
                                               'requires_tool': True, 'sounds': 'minecraft:basalt'})
        states = load('assets/tconstruct/blockstates/scorched_basin.json')['variants']
        self.assertEqual(set(states), {f'facing={d}' for d in ('north', 'south', 'east', 'west')})
        self.assertTrue(all(v['model'] == 'tconstruct:block/foundry/basin' for v in states.values()))
        self.assertEqual(load('assets/tconstruct/models/item/scorched_basin.json')['parent'],
                         'tconstruct:block/foundry/basin_covered')
        self.assertIn('tconstruct:scorched_basin',
                      (CONTENT / 'data/tconstruct/loot_table/blocks/scorched_basin.json').read_text())
        recipe = load('data/tconstruct/recipe/scorched_basin.json')
        self.assertEqual(recipe['key']['G'], 'tconstruct:gold_platform')
        self.assertEqual(recipe['result']['id'], 'tconstruct:scorched_basin')

    def test_machine_paths_and_require_cast(self):
        for file in ('source-machine-runtime.rs', 'source-machine-use.rs',
                     'source-machine-tick.rs', 'source-machine-remove.rs',
                     'source-machine-wire-send.rs', 'source-machine-wire-receive.rs',
                     'source-chute-hopper.rs', 'station-spill.rs',
                     'source-machine-shapes.rs', 'source-machine-geometry.rs',
                     'material-block-wire-type.rs', 'resource-tables.rs'):
            self.assertIn('tconstruct:scorched_basin', (MIXINS / file).read_text(), file)
        runtime = (MIXINS / 'source-machine-runtime.rs').read_text()
        self.assertIn('matches!(block, "tconstruct:scorched_table" | "tconstruct:scorched_basin") && casting.input.is_none()', runtime)
        self.assertIn('"tconstruct:seared_basin" | "tconstruct:scorched_basin" => Some("tconstruct:basin")', runtime)

if __name__ == '__main__':
    unittest.main()
