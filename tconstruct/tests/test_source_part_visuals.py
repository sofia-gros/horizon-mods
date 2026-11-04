"""Verify the pinned original part-model, sprite and render-info archive."""
import hashlib
import json
from pathlib import Path
import unittest
import zipfile

ROOT = Path(__file__).resolve().parents[1]


class SourcePartVisuals(unittest.TestCase):
    def test_forge_default_item_transform_is_pinned(self):
        path = ROOT / 'content/reference/forge-default-item-model.json'
        self.assertEqual(hashlib.sha256(path.read_bytes()).hexdigest(),
                         'c0d3d338fe6cc6ab5986e0be9b94677c36d40d0f8d4bd982e5e3999b612e6683')

    def test_english_overlay_retains_source_except_piggybackpack_warning(self):
        with zipfile.ZipFile(ROOT / "content/reference/tconstruct-part-visuals.zip") as archive:
            original = json.loads(archive.read("assets/tconstruct/lang/en_us.json"))
        overlay = json.loads((ROOT / "content/assets/tconstruct/lang/en_us.json").read_text())
        # The partial implementation deliberately replaces the pickup instruction
        # and labels the item so players are not promised mob carrying.
        intentional_overrides = {
            "item.tconstruct.piggy_backpack": "Piggybackpack (mob carrying unavailable)",
            "item.tconstruct.piggy_backpack.tooltip": "Mob carrying unavailable",
        }
        self.assertTrue(intentional_overrides.keys() <= original.keys())
        for key, value in original.items():
            with self.subTest(key=key):
                self.assertEqual(overlay.get(key), intentional_overrides.get(key, value))

    def test_archive_matches_reference_manifest(self):
        rows = (ROOT / 'docs/upstream-part-visuals.sha256').read_text().splitlines()
        expected = dict(row.split('  ', 1)[::-1] for row in rows)
        self.assertEqual(len(rows), 2179)
        self.assertEqual(len(expected), len(rows))
        with zipfile.ZipFile(ROOT / 'content/reference/tconstruct-part-visuals.zip') as archive:
            actual = {name: hashlib.sha256(archive.read(name)).hexdigest() for name in archive.namelist()}
        self.assertEqual(actual, expected)
