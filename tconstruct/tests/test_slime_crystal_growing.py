"""Source-parity checks for slime dirt blasting recipes."""
import json
from pathlib import Path
import unittest
import zipfile

ROOT = Path(__file__).resolve().parents[1]
DATA = ROOT / 'content/data/tconstruct'

class SlimeCrystalGrowing(unittest.TestCase):
    def test_all_four_recipes_match_pinned_source(self):
        with zipfile.ZipFile(ROOT / 'content/reference/tconstruct-recipes.zip') as archive:
            for kind in ('earth', 'sky', 'ichor', 'ender'):
                with self.subTest(kind=kind):
                    source = json.loads(archive.read(
                        f'data/tconstruct/recipes/common/slime/{kind}/crystal_growing.json'))
                    actual = json.loads((DATA / f'recipe/{kind}_crystal_growing.json').read_text())
                    self.assertEqual(actual, {
                        **source,
                        'ingredient': source['ingredient']['item'],
                        'result': {'id': source['result']},
                    })
                    self.assertEqual(actual['type'], 'minecraft:blasting')
                    self.assertEqual(actual['cookingtime'], 400)
                    self.assertEqual(actual['experience'], 0.2)
                    self.assertTrue((DATA / f'horizon/block/{kind}_slime_dirt.json').is_file())
                    self.assertEqual(actual['result']['id'], f'tconstruct:{kind}_slime_crystal')

if __name__ == '__main__':
    unittest.main()
