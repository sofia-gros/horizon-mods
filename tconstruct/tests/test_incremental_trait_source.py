"""Check the pinned incremental modifier modules used by the trait rebuild."""
import json
import unittest
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]

class IncrementalTraitSource(unittest.TestCase):
    def test_pinned_stat_boosts(self):
        expected = {
            'haste': ('tconstruct:mining_speed', 'add', 4.0),
            'sharpness': ('tconstruct:attack_damage', 'add', 0.75),
            'swiftstrike': ('tconstruct:attack_speed', 'multiply_base', 0.05),
            'quick_charge': ('tconstruct:draw_speed', 'multiply_base', 0.25),
            'trueshot': ('tconstruct:accuracy', 'add', 0.1),
        }
        with zipfile.ZipFile(ROOT / 'content/reference/tconstruct-modifiers.zip') as archive:
            for name, values in expected.items():
                definition = json.loads(archive.read(f'data/tconstruct/tinkering/modifiers/{name}.json'))
                modules = [m for m in definition['modules'] if m['type'] == 'tconstruct:stat_boost' and m['stat'] == values[0]]
                self.assertEqual(len(modules), 1, name)
                self.assertEqual((modules[0]['stat'], modules[0]['operation'], modules[0]['each_level']), values)

if __name__ == '__main__':
    unittest.main()
