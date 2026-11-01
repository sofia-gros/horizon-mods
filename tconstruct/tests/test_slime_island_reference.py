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

    def test_slime_island_biome_and_tree_references_are_retained(self):
        data_root = SOURCE.parent
        for definition in ('earth_slime_island', 'sky_slime_island',
                           'end_slime_island', 'blood_island', 'ocean_skyslime_island'):
            data = json.loads((SOURCE / f'structure/{definition}.json').read_text())
            biome = data['biomes'].removeprefix('#tconstruct:')
            self.assertTrue((data_root / f'tags/worldgen/biome/{biome}.json').is_file())
            for tree in data['trees']:
                feature = tree['data'].removeprefix('tconstruct:')
                self.assertTrue((SOURCE / f'configured_feature/{feature}.json').is_file())

    def test_overworld_slime_tree_height_settings(self):
        earth = json.loads((SOURCE / 'configured_feature/earth_slime_island_tree.json').read_text())['config']
        sky = json.loads((SOURCE / 'configured_feature/sky_slime_island_tree.json').read_text())['config']
        self.assertEqual((earth['base_height'], earth['random_height'], earth['can_double_height'], earth['has_vines']),
                         (4, 3, False, False))
        self.assertEqual((sky['base_height'], sky['random_height'], sky['can_double_height'], sky['has_vines']),
                         (5, 4, True, True))


if __name__ == '__main__':
    unittest.main()
