"""The MIT-licensed tag layer is the exact pinned upstream generated data."""
import hashlib
import json
from pathlib import Path
import unittest
import zipfile

ROOT = Path(__file__).resolve().parents[1]

class SourceTags(unittest.TestCase):
    def test_smeltery_block_tags_are_available_to_source_pack(self):
        with zipfile.ZipFile(ROOT / 'content/reference/tconstruct-block-tags.zip') as archive:
            names = archive.namelist()
            self.assertEqual(len(names), len(set(names)))
            self.assertTrue(all('/tags/blocks/' in name and name.endswith('.json') for name in names))
            wall = json.loads(archive.read('data/tconstruct/tags/blocks/smeltery/wall.json'))
            floor = json.loads(archive.read('data/tconstruct/tags/blocks/smeltery/floor.json'))
            self.assertIn('#tconstruct:seared_blocks', wall['values'])
            self.assertIn('#tconstruct:seared_blocks', floor['values'])
    def test_fluid_tags_match_pinned_manifest(self):
        rows = (ROOT / 'docs/upstream-fluid-tags.sha256').read_text().splitlines()
        expected = dict(row.split('  ', 1)[::-1] for row in rows)
        self.assertEqual(len(expected), 131)
        with zipfile.ZipFile(ROOT / 'content/reference/tconstruct-fluid-tags.zip') as archive:
            actual = {name: hashlib.sha256(archive.read(name)).hexdigest() for name in archive.namelist()}
        self.assertEqual(actual, expected)
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
