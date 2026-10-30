"""Check the pinned source tool visual archive against its hash manifest."""
import hashlib
from pathlib import Path
import unittest
import zipfile

ROOT = Path(__file__).resolve().parents[1]

class SourceToolVisuals(unittest.TestCase):
    def test_archive(self):
        rows = (ROOT / 'docs/upstream-tool-visuals.sha256').read_text().splitlines()
        with zipfile.ZipFile(ROOT / 'content/reference/tconstruct-tool-visuals.zip') as packed:
            self.assertEqual(len(packed.namelist()), len(rows))
            for row in rows:
                digest, name = row.split('  ', 1)
                self.assertEqual(hashlib.sha256(packed.read(name)).hexdigest(), digest)
