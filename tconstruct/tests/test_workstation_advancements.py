"""Source inventory criteria and parent chain for the first workstations."""
import json
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1] / 'content/data/tconstruct/advancement/tools'

class WorkstationAdvancements(unittest.TestCase):
    def test_inventory_criteria_and_parent_chain(self):
        for name, parent in [('part_builder', 'materials_and_you'), ('tinker_station', 'part_builder')]:
            with self.subTest(name=name):
                value = json.loads((ROOT / f'{name}.json').read_text())
                self.assertEqual(value['parent'], f'tconstruct:tools/{parent}')
                self.assertEqual(value['requirements'], [['crafted_block']])
                self.assertEqual(value['criteria']['crafted_block'], {
                    'trigger': 'minecraft:inventory_changed',
                    'conditions': {'items': [{'items': [f'tconstruct:{name}']}]},
                })
                self.assertEqual(value['display']['icon']['id'], f'tconstruct:{name}')
                self.assertNotIn('rewards', value)

if __name__ == '__main__':
    unittest.main()
