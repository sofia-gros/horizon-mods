"""Keep the original material-storage visual inputs byte-identical."""
import hashlib
import json
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]
VISUALS = ROOT / 'content/reference/tconstruct-storage-visuals'


class SourceStorageVisuals(unittest.TestCase):
    def test_manifest_and_all_source_files_match(self):
        rows = (ROOT / 'docs/upstream-storage-visuals.sha256').read_text().splitlines()
        expected = dict(row.split('  ', 1)[::-1] for row in rows)
        self.assertEqual(len(rows), 10)
        self.assertEqual(len(expected), len(rows))
        actual = {p.relative_to(VISUALS).as_posix(): hashlib.sha256(p.read_bytes()).hexdigest()
                  for p in VISUALS.rglob('*') if p.is_file()}
        self.assertEqual(actual, expected)

    def test_source_uses_material_cube_model(self):
        path = VISUALS / 'assets/tconstruct/models/block/storage/fake.json'
        model = json.loads(path.read_text())
        self.assertEqual(model['loader'], 'tconstruct:material_block')
        self.assertEqual(model['parent'], 'block/cube_all')
        self.assertEqual(model['material'], 'all')
        self.assertEqual(model['textures']['all'], 'tconstruct:block/storage/fallback')
