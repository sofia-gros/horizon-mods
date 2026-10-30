"""Verify the imported station layouts and their source manifest."""
import hashlib
import json
from pathlib import Path
import unittest
import zipfile

ROOT = Path(__file__).resolve().parents[1]

class SourceStationLayouts(unittest.TestCase):
    def test_pinned_layouts(self):
        rows = (ROOT / 'docs/upstream-station-layouts.sha256').read_text().splitlines()
        self.assertEqual(len(rows), 23)
        with zipfile.ZipFile(ROOT / 'content/reference/tconstruct-station-layouts.zip') as packed:
            self.assertEqual(len(packed.namelist()), len(rows))
            for row in rows:
                digest, name = row.split('  ', 1)
                value = packed.read(name)
                self.assertEqual(hashlib.sha256(value).hexdigest(), digest)
                layout = json.loads(value)
                self.assertIn('icon', layout)
                self.assertIn('input_slots', layout)
                self.assertIn('translation_key', layout)
