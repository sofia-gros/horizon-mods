"""Pinned Travelers' Gear acquisition criterion and parent."""
import json
import unittest
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]/'content/data/tconstruct/advancement/tools'
class TravelersGearAdvancement(unittest.TestCase):
    def test_requires_each_distinct_piece(self):
        data=json.loads((ROOT/'travelers_gear.json').read_text())
        self.assertEqual(data['parent'],'tconstruct:tools/tinker_station')
        self.assertTrue((ROOT/'tinker_station.json').is_file())
        pieces=('helmet','chestplate','leggings','boots')
        self.assertEqual(data['requirements'],[[f'crafted_{p}'] for p in pieces])
        self.assertEqual(set(data['criteria']),{f'crafted_{p}' for p in pieces})
        for part in pieces:
            self.assertEqual(data['criteria'][f'crafted_{part}'],{
                'trigger':'minecraft:inventory_changed',
                'conditions':{'items':[{'items':[f'tconstruct:travelers_{part}']}]}})
        self.assertEqual(data['display']['icon']['id'],'tconstruct:travelers_helmet')
        self.assertNotIn('rewards',data)
if __name__=='__main__': unittest.main()
