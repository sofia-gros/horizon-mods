"""The MIT-licensed tag layer is the exact pinned upstream generated data."""
import hashlib
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]

class SourceTags(unittest.TestCase):
    def test_tag_layer_matches_reference_manifest(self):
        root = ROOT / 'content/reference/tconstruct-tags'
        rows = (ROOT / 'docs/upstream-tags.sha256').read_text().splitlines()
        self.assertEqual(len(rows), 427)
        expected = {}
        for row in rows:
            digest, path = row.split('  ', 1)
            expected[path] = digest
        actual = {p.relative_to(root).as_posix(): hashlib.sha256(p.read_bytes()).hexdigest()
                  for p in root.rglob('*.json')}
        self.assertEqual(actual, expected)
