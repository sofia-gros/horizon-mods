"""Pinned Cast Collector acquisition conditions and packaging."""
import json
import sys
import tempfile
import unittest
import zipfile
from pathlib import Path
MOD = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(MOD / 'tools'))
from package_hmod import package
NAMES = ['ingot', 'nugget', 'gem', 'rod', 'repair_kit', 'pick_head',
         'small_axe_head', 'small_blade', 'adze_head', 'hammer_head',
         'broad_blade', 'broad_axe_head', 'large_plate', 'tool_binding',
         'tough_binding', 'tool_handle', 'tough_handle', 'bow_limb',
         'bow_grip', 'helmet_plating', 'chestplate_plating',
         'leggings_plating', 'boots_plating', 'maille']

class CastCollectorAdvancement(unittest.TestCase):
    def test_source_criteria_and_display(self):
        data = json.loads((MOD / 'content/data/tconstruct/advancement/smeltery/cast_collector.json').read_text())
        self.assertEqual(data['parent'], 'tconstruct:smeltery/gold_casting')
        self.assertEqual(data['requirements'], [[name] for name in NAMES])
        self.assertEqual(set(data['criteria']), set(NAMES))
        for name in NAMES:
            self.assertEqual(data['criteria'][name], {
                'trigger': 'minecraft:inventory_changed',
                'conditions': {'items': [{'items': [f'tconstruct:{name}_cast']}]},
            })
        self.assertEqual(data['display']['frame'], 'goal')
        self.assertTrue(data['display']['show_toast'])
        self.assertTrue(data['display']['announce_to_chat'])
        self.assertEqual(data['display']['icon'], {'id': 'tconstruct:hammer_head_cast'})
        self.assertNotIn('rewards', data)

    def test_package_contains_advancement_and_all_item_declarations(self):
        with tempfile.TemporaryDirectory() as tmp:
            output = Path(tmp) / 'tconstruct.hmod'
            package(output)
            with zipfile.ZipFile(output) as packed:
                files = set(packed.namelist())
                self.assertIn('data/tconstruct/advancement/smeltery/cast_collector.json', files)
                for name in NAMES:
                    self.assertIn(f'data/tconstruct/horizon/item/{name}_cast.json', files)

if __name__ == '__main__':
    unittest.main()
