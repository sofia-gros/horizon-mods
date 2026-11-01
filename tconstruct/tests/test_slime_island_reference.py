"""Pinned island inputs are intact; these tests do not assert world generation."""
import gzip
import json
import unittest
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / 'content/reference/slime-islands/data/tconstruct/worldgen'
TEMPLATES = ROOT / 'content/reference/slime-islands-templates.zip'


class SlimeIslandReferenceTest(unittest.TestCase):
    def test_overworld_sky_set_keeps_source_frequency_and_weights(self):
        data = json.loads((SOURCE / 'structure_set/overworld_sky_island.json').read_text())
        self.assertEqual(data['placement'], {
            'type': 'minecraft:random_spread', 'frequency': 0.5,
            'salt': 14357800, 'separation': 4, 'spacing': 35,
        })
        self.assertEqual([(v['structure'], v['weight']) for v in data['structures']], [
            ('tconstruct:sky_slime_island', 4), ('tconstruct:clay_island', 1),
        ])

    def test_template_references_exist_and_are_gzip_nbt(self):
        with zipfile.ZipFile(TEMPLATES) as archive:
            for variant, definition in [('earth', 'earth_slime_island'),
                                        ('sky', 'sky_slime_island'),
                                        ('ender', 'end_slime_island'),
                                        ('blood', 'blood_island')]:
                data = json.loads((SOURCE / f'structure/{definition}.json').read_text())
                self.assertEqual(data['type'], 'tconstruct:island')
                self.assertEqual(len(data['templates']), 5)
                for entry in data['templates']:
                    name = entry['data'].removeprefix('tconstruct:')
                    self.assertTrue(name.startswith(f'islands/{variant}/'))
                    raw = gzip.decompress(archive.read(f'data/tconstruct/structures/{name}.nbt'))
                    self.assertEqual(raw[0], 10)  # Named NBT compound root.


if __name__ == '__main__':
    unittest.main()
