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

    def test_registered_fallback_uses_source_textures(self):
        source = VISUALS / 'assets/tconstruct/textures/block/storage'
        target = ROOT / 'content/assets/tconstruct/textures/block/storage'
        self.assertEqual({p.name: p.read_bytes() for p in source.glob('*.png')},
                         {p.name: p.read_bytes() for p in target.glob('*.png')})
        block = json.loads((ROOT / 'content/data/tconstruct/horizon/block/fake_storage_block.json').read_text())
        self.assertEqual(block['template'], 'minecraft:iron_block')
        self.assertEqual(block['properties']['hardness'], 5.0)
        self.assertEqual(block['properties']['explosion_resistance'], 5.0)
        model = json.loads((ROOT / 'content/assets/tconstruct/models/block/storage/fake.json').read_text())
        self.assertEqual(model['textures']['all'], 'tconstruct:block/storage/fallback')
        for name in ('mineable/pickaxe', 'needs_iron_tool'):
            tag = json.loads((ROOT / f'content/data/minecraft/tags/block/{name}.json').read_text())
            self.assertIn('tconstruct:fake_storage_block', tag['values'])
