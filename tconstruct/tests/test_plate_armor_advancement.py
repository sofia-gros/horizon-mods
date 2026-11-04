"""Pinned PlateArmor acquisition conditions and packaging."""
import json
import sys
import tempfile
import unittest
import zipfile
from pathlib import Path

MOD = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(MOD / 'tools'))
from package_hmod import package

class PlateArmorAdvancement(unittest.TestCase):
    def test_source_criteria_and_display(self):
        data = json.loads((MOD / 'content/data/tconstruct/advancement/foundry/plate_armor.json').read_text())
        criteria = {'crafted_helmet': 'plate_helmet', 'crafted_chestplate': 'plate_chestplate',
                    'crafted_leggings': 'plate_leggings', 'crafted_boots': 'plate_boots'}
        self.assertEqual(data['parent'], 'tconstruct:foundry/blaze')
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
        self.assertEqual(data['display']['icon'], {'id': 'tconstruct:plate_chestplate'})
        self.assertNotIn('rewards', data)

    def test_package_contains_advancement_and_all_item_declarations(self):
        with tempfile.TemporaryDirectory() as tmp:
            output = Path(tmp) / 'tconstruct.hmod'
            package(output)
            with zipfile.ZipFile(output) as packed:
                files = set(packed.namelist())
                self.assertIn('data/tconstruct/advancement/foundry/plate_armor.json', files)
                for name in ['plate_helmet', 'plate_chestplate', 'plate_leggings', 'plate_boots']:
                    self.assertIn(f'data/tconstruct/horizon/item/{name}.json', files)

if __name__ == '__main__':
    unittest.main()
