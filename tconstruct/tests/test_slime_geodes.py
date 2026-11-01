"""Check source geode variants, material inputs and derived recipes."""
import json
import unittest
from pathlib import Path
from zipfile import ZipFile

ROOT = Path(__file__).resolve().parents[1]
DATA = ROOT / 'content/data/tconstruct'

class SlimeGeodes(unittest.TestCase):
    def test_source_placement_and_cluster_sets(self):
        expected = {
            'earth': (256, {'above_bottom': 6}, {'above_bottom': 54}, 'steel'),
            'ichor': (52, {'below_top': 48}, {'below_top': 16}, 'cobalt'),
            'ender': (256, {'above_bottom': 16}, {'above_bottom': 64}, 'knightmetal'),
        }
        for color, (rarity, low, high, metal) in expected.items():
            with self.subTest(color=color):
                placed = json.loads((DATA / f'worldgen/placed_feature/{color}_geode.json').read_text())
                feature = json.loads((DATA / f'worldgen/configured_feature/{color}_geode.json').read_text())
                self.assertEqual(placed['placement'][0]['chance'], rarity)
                self.assertEqual(placed['placement'][2]['height']['min_inclusive'], low)
                self.assertEqual(placed['placement'][2]['height']['max_inclusive'], high)
                inner = feature['blocks']['inner_placements']
                self.assertIn(f'tconstruct:{color}_slime_crystal_cluster', inner)
                self.assertIn(f'tconstruct:{metal}_cluster', inner)
                self.assertEqual(len(inner), 5)

    def test_crafting_and_source_material_recipes(self):
        with ZipFile(ROOT / 'content/reference/tconstruct-recipes.zip') as source:
            for color, material in [('earth', 'earthslime'), ('sky', 'skyslime'), ('ichor', 'ichor'), ('ender', 'enderslime')]:
                with self.subTest(color=color):
                    src = json.loads(source.read(f'data/tconstruct/recipes/common/slime/{color}/crystal_smelting.json'))
                    out = json.loads((DATA / f'recipe/{color}_crystal_smelting.json').read_text())
                    self.assertEqual(out['ingredient'], src['ingredient']['item'])
                    self.assertEqual(out['result']['id'], src['result'])
                    self.assertEqual((out['cookingtime'], out['experience']), (src['cookingtime'], src['experience']))
                    crafted = json.loads((DATA / f'recipe/{color}_crystal_block.json').read_text())
                    self.assertEqual(crafted['key']['#'], f'tconstruct:{color}_slime_crystal')
                    self.assertEqual(crafted['result']['id'], f'tconstruct:{color}_slime_crystal_block')
                    recipe = json.loads(source.read(f'data/tconstruct/recipes/tools/materials/{material}.json'))
                    self.assertEqual(recipe['ingredient']['item'], f'tconstruct:{color}_slime_crystal')
                    self.assertEqual(recipe['material'], f'tconstruct:{material}')

if __name__ == '__main__':
    unittest.main()
