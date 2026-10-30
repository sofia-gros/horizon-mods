"""Validate the pinned original reusable cast definitions and assets."""
import hashlib
import json
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]

class GoldCasts(unittest.TestCase):
    def test_all_source_models_and_sprites(self):
        rows = (ROOT / 'docs/upstream-gold-casts.sha256').read_text().splitlines()
        self.assertEqual(len(rows), 58)
        for row in rows:
            digest, path = row.split('  ', 1)
            self.assertEqual(hashlib.sha256((ROOT / path).read_bytes()).hexdigest(), digest, path)
        tag = json.loads((ROOT / 'content/reference/tconstruct-tags/data/tconstruct/tags/items/casts/gold.json').read_text())
        self.assertEqual(len(tag['values']), 29)
        for item in tag['values']:
            short = item.split(':', 1)[1]
            model = json.loads((ROOT / 'content/assets/tconstruct/models/item' / (short + '.json')).read_text())
            texture = model['textures']['layer0'].split(':', 1)[1]
            self.assertTrue((ROOT / 'content/assets/tconstruct/textures' / (texture + '.png')).is_file())
