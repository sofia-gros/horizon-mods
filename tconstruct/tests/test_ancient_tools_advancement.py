"""Pinned Ancient Tools acquisition criteria and their AND semantics."""
import json
import sys
import tempfile
import unittest
import zipfile
from pathlib import Path

MOD = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(MOD / 'tools'))
from package_hmod import package

class AncientToolsAdvancement(unittest.TestCase):
    def test_source_criteria_and_display(self):
        data = json.loads((MOD / 'content/data/tconstruct/advancement/world/ancient_tools.json').read_text())
        names = ['melting_pan', 'war_pick', 'battlesign', 'swasher']
        self.assertEqual(data['parent'], 'tconstruct:world/tinkers_gadgetry')
        self.assertEqual(data['requirements'], [[name] for name in names])
        self.assertEqual(set(data['criteria']), set(names))
        for name in names:
            self.assertEqual(data['criteria'][name], {
                'trigger': 'minecraft:inventory_changed',
                'conditions': {'items': [{'items': [f'tconstruct:{name}']}]},
            })
        self.assertEqual(data['display']['frame'], 'challenge')
        self.assertTrue(data['display']['show_toast'])
        self.assertTrue(data['display']['announce_to_chat'])
        self.assertEqual(data['display']['icon'], {'id': 'tconstruct:battlesign'})
        self.assertNotIn('rewards', data)

    def test_package_contains_advancement_and_all_item_declarations(self):
        with tempfile.TemporaryDirectory() as tmp:
            output = Path(tmp) / 'tconstruct.hmod'
            package(output)
            with zipfile.ZipFile(output) as packed:
                files = set(packed.namelist())
                self.assertIn('data/tconstruct/advancement/world/ancient_tools.json', files)
                for name in ['melting_pan', 'war_pick', 'battlesign', 'swasher']:
                    self.assertIn(f'data/tconstruct/horizon/item/{name}.json', files)

if __name__ == '__main__':
    unittest.main()
