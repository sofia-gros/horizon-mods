"""Pinned Anvil acquisition criteria and the Smeltery-opening parent."""
import json
import unittest
from pathlib import Path
ROOT = Path(__file__).resolve().parents[1] / 'content/data/tconstruct/advancement/smeltery'

class TinkersAnvilAdvancement(unittest.TestCase):
    def test_either_registered_anvil_meets_source_inventory_criterion(self):
        data = json.loads((ROOT / 'tinkers_anvil.json').read_text())
        self.assertEqual(data['parent'], 'tconstruct:smeltery/structure')
        self.assertEqual(data['requirements'], [['crafted_overworld', 'crafted_nether']])
        self.assertEqual(set(data['criteria']), {'crafted_overworld', 'crafted_nether'})
        for criterion, item in [('crafted_overworld', 'tconstruct:tinkers_anvil'),
                                ('crafted_nether', 'tconstruct:scorched_anvil')]:
            self.assertEqual(data['criteria'][criterion], {
                'trigger': 'minecraft:inventory_changed',
                'conditions': {'items': [{'items': [item]}]},
            })
        self.assertEqual(data['display']['icon']['id'], 'tconstruct:tinkers_anvil')
        self.assertEqual(data['display']['frame'], 'goal')
        self.assertNotIn('rewards', data)

if __name__ == '__main__': unittest.main()
