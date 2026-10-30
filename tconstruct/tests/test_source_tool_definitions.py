"""Source tool definition archive and byte manifest remain identical."""
import hashlib
from pathlib import Path
import unittest
import zipfile

ROOT = Path(__file__).resolve().parents[1]

class SourceToolDefinitions(unittest.TestCase):
    def test_pinned_archive(self):
        archive = ROOT / 'content/reference/tconstruct-tool-definitions.zip'
        self.assertEqual(hashlib.sha256(archive.read_bytes()).hexdigest(), '12ddbf9968c4af58d816dfea1ed72f146e46a07830d6d2183d26f4495855d86f')
        rows = (ROOT / 'docs/upstream-tool-definitions.sha256').read_text().splitlines()
        self.assertEqual(len(rows), 45)
        with zipfile.ZipFile(archive) as packed:
            self.assertEqual(len(packed.namelist()), len(rows))
            for row in rows:
                digest, name = row.split('  ', 1)
                self.assertEqual(hashlib.sha256(packed.read(name)).hexdigest(), digest)
