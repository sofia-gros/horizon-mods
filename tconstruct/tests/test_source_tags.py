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
    def test_modifier_resource_archives_are_complete_and_packaged(self):
        expected = dict(row.split('  ', 1)[::-1] for row in (ROOT / 'docs/upstream-modifiers.sha256').read_text().splitlines())
        self.assertEqual(len(expected), 366)  # 308 definitions and 58 additional tags
        with zipfile.ZipFile(ROOT / 'content/reference/tconstruct-modifiers.zip') as archive:
            names = archive.namelist()
            self.assertEqual(len(names), len(set(names)))
            actual = {name: hashlib.sha256(archive.read(name)).hexdigest() for name in names}
            abilities = json.loads(archive.read('data/tconstruct/tinkering/tags/modifiers/abilities.json'))
            general = json.loads(archive.read('data/tconstruct/tinkering/tags/modifiers/upgrades/general.json'))
        self.assertEqual(actual, expected)
        self.assertIn('#tconstruct:abilities/general', abilities['values'])
        self.assertIn({'id': 'tconstruct:the_one_probe', 'required': False}, general['values'])
        retained = ROOT / 'content/reference/tconstruct-tags/data/tconstruct/tinkering/tags/modifiers/durability/bypass_reinforced.json'
        self.assertTrue(retained.is_file())

    def test_pinned_modifier_redirect_forms(self):
        with zipfile.ZipFile(ROOT / 'content/reference/tconstruct-modifiers.zip') as archive:
            definitions = {name: json.loads(archive.read(name)) for name in archive.namelist()
                           if '/tinkering/modifiers/' in name}
        redirects = {name.rsplit('/', 1)[-1]: value['redirects'] for name, value in definitions.items()
                     if 'redirects' in value}
        self.assertEqual(redirects, {
            'frosttouch.json': [{'id': 'tconstruct:slow_bones'}],
            'lightspeed_armor.json': [{'id': 'tconstruct:lightspeed'}],
            'path_maker.json': [{'id': 'tconstruct:pathing'}],
            'plowing.json': [{'id': 'tconstruct:tilling'}],
            'sharpweight.json': [{'id': 'tconstruct:heavy'}],
            'sturdy.json': [{'id': 'tconstruct:ductile'}],
        })
        self.assertEqual(sum('condition' in value for value in definitions.values()), 3)

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
        self.assertEqual(len(rows), 428)
        expected = {}
        for row in rows:
            digest, path = row.split('  ', 1)
            expected[path] = digest
        actual = {p.relative_to(root).as_posix(): hashlib.sha256(p.read_bytes()).hexdigest()
                  for p in root.rglob('*.json')}
        self.assertEqual(actual, expected)
