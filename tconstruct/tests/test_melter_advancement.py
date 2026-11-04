"""Pinned source Melter placement advancement, adapted to host icon format."""
import json
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


class MelterAdvancement(unittest.TestCase):
    def test_placed_blocks_and_alternative_heat_source(self):
        v = json.loads((ROOT / 'content/data/tconstruct/advancement/smeltery/melter.json').read_text())
        self.assertEqual(v['parent'], 'tconstruct:smeltery/puny_smelting')
        parent = json.loads((ROOT / 'content/data/tconstruct/advancement/smeltery/puny_smelting.json').read_text())
        self.assertEqual(parent['parent'], 'tconstruct:tools/materials_and_you')
        self.assertTrue((ROOT / 'content/data/tconstruct/advancement/tools/materials_and_you.json').is_file())
        self.assertEqual(v['display']['icon'], {'id': 'tconstruct:seared_melter'})
        self.assertEqual(v['requirements'], [
            ['seared_melter'], ['seared_table'], ['seared_basin'], ['seared_faucet'],
            ['seared_heater', 'seared_fuel_tank', 'seared_fuel_gauge',
             'seared_ingot_tank', 'seared_ingot_gauge'],
        ])
        names = {name for group in v['requirements'] for name in group}
        self.assertEqual(set(v['criteria']), names)
        for name in names:
            self.assertEqual(v['criteria'][name], {
                'trigger': 'minecraft:placed_block',
                'conditions': {'location': [{
                    'block': 'tconstruct:' + name,
                    'condition': 'minecraft:block_state_property',
                }]},
            })
        self.assertNotIn('rewards', v)


if __name__ == '__main__':
    unittest.main()
