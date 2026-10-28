import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

SCRIPT = Path(__file__).resolve().parents[1] / 'tools/resources.py'

class ResourceOverlay(unittest.TestCase):
    def test_overlay_keeps_input_and_registers_items(self):
        with tempfile.TemporaryDirectory() as tmp:
            src, out = Path(tmp) / 'src', Path(tmp) / 'out'
            src.mkdir()
            raw = {'minecraft:flint': {}, 'minecraft:stick': {},
                   'minecraft:stone_pickaxe': {'minecraft:tool': {'rules': [{}, {'speed': 4}]}}}
            (src / 'items.json').write_text(json.dumps(raw))
            (src / 'command-registries.json').write_text(json.dumps({'items': list(raw), 'by_registry': {'minecraft:item': {'ids': list(raw), 'tags': []}}}))
            (src / 'registries.json').write_text(json.dumps({'minecraft:item': {'entries': {'minecraft:air': {'protocol_id': 0}}}}))
            (src / 'assets').mkdir()
            subprocess.run([sys.executable, str(SCRIPT), str(src), str(out)], check=True)
            self.assertEqual(json.loads((src / 'items.json').read_text()), raw)
            items = json.loads((out / 'items.json').read_text())
            ids = json.loads((out / 'registries.json').read_text())['minecraft:item']['entries']
            names = ['tconstruct:flint_pick_head', 'tconstruct:wood_binding', 'tconstruct:wood_handle', 'tconstruct:flint_pickaxe']
            self.assertTrue(all(name in items and name in ids for name in names))
            command_ids = json.loads((out / 'command-registries.json').read_text())['by_registry']['minecraft:item']['ids']
            self.assertTrue(all(name in command_ids for name in names))
            self.assertEqual(items[names[-1]]['minecraft:max_damage'], 85)
            self.assertEqual(items[names[-1]]['minecraft:tool']['rules'][-1]['speed'], 3.5)
            self.assertEqual(len({ids[name]['protocol_id'] for name in names}), 4)
            self.assertTrue((out / 'assets').is_symlink())
            rejected = subprocess.run([sys.executable, str(SCRIPT), str(src), str(out)], capture_output=True)
            self.assertNotEqual(rejected.returncode, 0)

if __name__ == '__main__':
    unittest.main()
