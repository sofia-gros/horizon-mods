"""Pinned source cobalt placement and reachable cooking resources."""
import json
import unittest
from pathlib import Path
from zipfile import ZipFile

ROOT = Path(__file__).resolve().parents[1]
DATA = ROOT / 'content/data/tconstruct'

class CobaltProgression(unittest.TestCase):
    def test_source_veins(self):
        for name, size, count, kind, low, high in [
            ('small', 4, 5, 'uniform', {'above_bottom': 8}, {'below_top': 8}),
            ('large', 6, 3, 'trapezoid', {'absolute': 8}, {'absolute': 32}),
        ]:
            with self.subTest(name=name):
                feature = json.loads((DATA / f'worldgen/configured_feature/cobalt_ore_{name}.json').read_text())
                placed = json.loads((DATA / f'worldgen/placed_feature/cobalt_ore_{name}.json').read_text())
                self.assertEqual((feature['type'], feature['size'], feature['discard_chance_on_air_exposure']), ('minecraft:ore', size, 0.0))
                self.assertEqual(feature['targets'], [{'state':'tconstruct:cobalt_ore','target':{'block':'minecraft:netherrack','predicate_type':'minecraft:block_match'}}])
                self.assertEqual(placed['placement'][0]['count'], count)
                height = placed['placement'][2]['height']
                self.assertEqual((height['type'], height['min_inclusive'], height['max_inclusive']), ('minecraft:'+kind, low, high))
                self.assertEqual(placed['placement'][-1]['type'], 'minecraft:biome')

    def test_cobalt_blasting_matches_source(self):
        with ZipFile(ROOT / 'content/reference/tconstruct-recipes.zip') as source:
            for filename, original, ingredient in [('cobalt_raw_blasting','cobalt_ingot_blasting','raw_cobalt'),('cobalt_ore_blasting','cobalt_ingot_blasting','cobalt_ore'),('cobalt_shard_blasting','cobalt_nugget_blasting','cobalt_shard')]:
                src=json.loads(source.read(f'data/tconstruct/recipes/common/materials/{original}.json'))
                out=json.loads((DATA / f'recipe/{filename}.json').read_text())
                self.assertEqual(out['ingredient'], 'tconstruct:'+ingredient)
                self.assertEqual((out['type'],out['cookingtime'],out['experience'],out['result']['id']), (src['type'],src['cookingtime'],src['experience'],src['result']))

if __name__ == '__main__': unittest.main()
