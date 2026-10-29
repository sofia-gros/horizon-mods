"""Check the pinned original dummy plating models and sprites."""
import hashlib
import json
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]

class DummyPlating(unittest.TestCase):
    def test_source_assets(self):
        rows = (ROOT / 'docs/upstream-dummy-plating.sha256').read_text().splitlines()
        self.assertEqual(len(rows), 8)
        for row in rows:
            digest, name = row.split('  ', 1)
            self.assertEqual(hashlib.sha256((ROOT / name).read_bytes()).hexdigest(), digest, name)
        for part in ('helmet', 'chestplate', 'leggings', 'boots'):
            model = json.loads((ROOT / 'content/assets/tconstruct/models/item' / (part + '_plating_dummy.json')).read_text())
            self.assertEqual(model['textures']['layer0'], 'tconstruct:item/tool/parts/plating_' + part)
