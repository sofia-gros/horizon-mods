"""Pinned Slimesuit acquisition conditions and packaging."""
import json
import sys
import tempfile
import unittest
import zipfile
from pathlib import Path

MOD = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(MOD / 'tools'))
from package_hmod import package

class SlimesuitAdvancement(unittest.TestCase):
    def test_source_criteria_and_display(self):
        data = json.loads((MOD / 'content/data/tconstruct/advancement/world/slimesuit.json').read_text())
        criteria = {'crafted_helmet': 'slime_helmet', 'crafted_chestplate': 'slimy_chestplate',
                    'crafted_leggings': 'slime_leggings', 'crafted_boots': 'slime_boots'}
        self.assertEqual(data['parent'], 'tconstruct:world/sky_island')
        self.assertEqual(data['requirements'], [[name] for name in criteria])
        self.assertEqual(set(data['criteria']), set(criteria))
        for criterion, item in criteria.items():
            self.assertEqual(data['criteria'][criterion], {
                'trigger': 'minecraft:inventory_changed',
                'conditions': {'items': [{'items': [f'tconstruct:{item}']}]},
            })
        self.assertEqual(data['display']['frame'], 'goal')
        self.assertTrue(data['display']['show_toast'])
        self.assertTrue(data['display']['announce_to_chat'])
        self.assertEqual(data['display']['icon'], {'id': 'tconstruct:slimy_chestplate'})
        self.assertNotIn('rewards', data)

    def test_package_contains_advancement_and_all_item_declarations(self):
        with tempfile.TemporaryDirectory() as tmp:
            output = Path(tmp) / 'tconstruct.hmod'
            package(output)
            with zipfile.ZipFile(output) as packed:
                files = set(packed.namelist())
                self.assertIn('data/tconstruct/advancement/world/slimesuit.json', files)
                for name in ['slime_helmet', 'slimy_chestplate', 'slime_leggings', 'slime_boots']:
                    self.assertIn(f'data/tconstruct/horizon/item/{name}.json', files)

if __name__ == '__main__':
    unittest.main()
