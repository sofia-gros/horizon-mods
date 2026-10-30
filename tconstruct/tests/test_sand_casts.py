"""Validate the pinned original sand and red-sand cast assets."""
import hashlib
import json
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]

class SandCasts(unittest.TestCase):
    def test_source_models_and_sprites(self):
        rows = (ROOT / 'docs/upstream-sand-casts.sha256').read_text().splitlines()
        self.assertEqual(len(rows), 116)
        for row in rows:
            digest, path = row.split('  ', 1)
            self.assertEqual(hashlib.sha256((ROOT / path).read_bytes()).hexdigest(), digest, path)
        base = ROOT / 'content/reference/tconstruct-tags/data/tconstruct/tags/items/casts'
        for kind in ('sand', 'red_sand'):
            values = json.loads((base / (kind + '.json')).read_text())['values']
            self.assertEqual(len(values), 29)
            for item in values:
                short = item.split(':', 1)[1]
                model = json.loads((ROOT / 'content/assets/tconstruct/models/item' / (short + '.json')).read_text())
                texture = model['textures']['layer0'].split(':', 1)[1]
                self.assertTrue((ROOT / 'content/assets/tconstruct/textures' / (texture + '.png')).is_file())
