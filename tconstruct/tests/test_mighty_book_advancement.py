"""Pinned Mighty Smelting book acquisition criterion."""
import json
import unittest
from pathlib import Path
ROOT = Path(__file__).resolve().parents[1] / 'content/data/tconstruct'
class MightyBookAdvancement(unittest.TestCase):
    def test_inventory_trigger_and_melter_parent(self):
        data=json.loads((ROOT/'advancement/smeltery/mighty_smelting.json').read_text())
        self.assertEqual(data['parent'], 'tconstruct:smeltery/melter')
        self.assertTrue((ROOT/'advancement/smeltery/melter.json').is_file())
        self.assertEqual(data['requirements'], [['crafted_book']])
        self.assertEqual(data['criteria'], {'crafted_book': {'trigger':'minecraft:inventory_changed',
            'conditions': {'items':[{'items':['tconstruct:mighty_smelting']}]}}})
        self.assertEqual(data['display']['icon'], {'id':'tconstruct:mighty_smelting'})
        self.assertNotIn('rewards', data)
        self.assertTrue((ROOT/'horizon/item/mighty_smelting.json').is_file())
if __name__=='__main__':
    unittest.main()
