"""Pinned Tool Forge acquisition criteria and their AND semantics."""
import json
import unittest
import sys
import tempfile
import zipfile
from pathlib import Path
MOD = Path(__file__).resolve().parents[1]
ROOT = MOD / 'content/data/tconstruct/advancement/smeltery'
sys.path.insert(0, str(MOD / 'tools'))
from package_hmod import package

class ToolForgeAdvancement(unittest.TestCase):
    def test_each_source_tool_has_its_own_inventory_criterion(self):
        data = json.loads((ROOT / 'tool_forge.json').read_text())
        names = ['sledge_hammer', 'vein_hammer', 'excavator', 'broad_axe',
                 'scythe', 'cleaver', 'longbow', 'javelin']
        self.assertEqual(data['parent'], 'tconstruct:smeltery/tinkers_anvil')
        self.assertTrue((ROOT / 'tinkers_anvil.json').is_file())
        self.assertEqual(data['requirements'], [[name] for name in names])
        self.assertEqual(set(data['criteria']), set(names))
        for name in names:
            self.assertEqual(data['criteria'][name], {
                'trigger': 'minecraft:inventory_changed',
                'conditions': {'items': [{'items': [f'tconstruct:{name}']}]},
            })
        self.assertEqual(data['display']['frame'], 'challenge')
        self.assertEqual(data['display']['icon'], {'id': 'tconstruct:vein_hammer'})
        self.assertNotIn('rewards', data)

    def test_packaged_criteria_items_are_declared(self):
        names = ['sledge_hammer', 'vein_hammer', 'excavator', 'broad_axe',
                 'scythe', 'cleaver', 'longbow', 'javelin']
        with tempfile.TemporaryDirectory() as tmp:
            output = Path(tmp) / 'tconstruct.hmod'
            package(output)
            with zipfile.ZipFile(output) as packed:
                files = set(packed.namelist())
                self.assertIn('data/tconstruct/advancement/smeltery/tool_forge.json', files)
                for name in names:
                    self.assertIn(f'data/tconstruct/horizon/item/{name}.json', files)

if __name__ == '__main__': unittest.main()
