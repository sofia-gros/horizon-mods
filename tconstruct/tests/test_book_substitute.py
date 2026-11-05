"""Pinned leather-free book recipe and data-only early-guide reachability."""
from collections import Counter
import json
from pathlib import Path
import sys
import tempfile
import unittest
from zipfile import ZipFile

from test_survival_reachability import VANILLA, closure, tag_items

ROOT = Path(__file__).resolve().parents[1]
CONTENT = ROOT / 'content'
RECIPE = 'data/tconstruct/recipe/book_substitute.json'
TAG = 'data/forge/tags/item/slimeballs.json'
sys.path.insert(0, str(ROOT / 'tools'))
from package_hmod import package


class BookSubstitute(unittest.TestCase):
    def test_recipe_preserves_source_counts_and_tag(self):
        with ZipFile(CONTENT / 'reference/tconstruct-recipes.zip') as source:
            original = json.loads(source.read('data/tconstruct/recipes/tables/book_substitute.json'))
        expected = dict(original)
        expected['ingredients'] = ['#' + v['tag'] if 'tag' in v else v['item'] for v in original['ingredients']]
        expected['result'] = {'id': original['result']['item']}
        actual = json.loads((CONTENT / RECIPE).read_text())
        self.assertEqual(actual, expected)
        self.assertEqual(actual['type'], 'minecraft:crafting_shapeless')
        self.assertEqual(Counter(actual['ingredients']), {
            'minecraft:paper': 3, '#forge:slimeballs': 1, 'tconstruct:pattern': 2,
        })
        self.assertEqual(actual['result'], {'id': 'minecraft:book'})

    def test_aggregate_tag_preserves_both_source_layers(self):
        with ZipFile(CONTENT / 'reference/forge-47.4.0-data.zip') as forge:
            vanilla = json.loads(forge.read('data/forge/tags/items/slimeballs.json'))
        original = json.loads((CONTENT / 'reference/tconstruct-tags/data/forge/tags/items/slimeballs.json').read_text())
        self.assertEqual(json.loads((CONTENT / TAG).read_text()), {'values': vanilla['values'] + original['values']})
        # Keep nested tags so subtype additions/replacements still affect this recipe.
        self.assertEqual(original['values'], [f'#forge:slimeball/{kind}' for kind in ('earth', 'sky', 'ichor', 'ender')])
        self.assertEqual(tag_items('forge:slimeballs'), {
            'minecraft:slime_ball', 'tconstruct:sky_slime_ball',
            'tconstruct:ichor_slime_ball', 'tconstruct:ender_slime_ball',
        })

    def test_guide_route_needs_paper_and_slime_but_not_leather(self):
        recipes = [json.loads(p.read_text()) for p in (CONTENT / 'data/tconstruct/recipe').glob('*.json')]
        for ball in tag_items('forge:slimeballs'):
            with self.subTest(ball=ball):
                reachable = closure(recipes, VANILLA | {'minecraft:paper', ball})
                self.assertIn('minecraft:book', reachable)
                self.assertIn('tconstruct:materials_and_you', reachable)
                self.assertNotIn('minecraft:leather', reachable)
        for missing in ('minecraft:paper', 'minecraft:slime_ball'):
            starting = (VANILLA | {'minecraft:paper', 'minecraft:slime_ball'}) - {missing}
            self.assertNotIn('minecraft:book', closure(recipes, starting))

    def test_packaged_recipe_tag_and_mod_inputs(self):
        with tempfile.TemporaryDirectory() as tmp:
            output = Path(tmp) / 'tconstruct.hmod'
            package(output)
            with ZipFile(output) as archive:
                for path in (RECIPE, TAG, 'data/tconstruct/recipe/materials_and_you.json'):
                    self.assertEqual(archive.read(path), (CONTENT / path).read_bytes())
                for kind in ('earth', 'sky', 'ichor', 'ender'):
                    self.assertIn(f'data/forge/tags/item/slimeball/{kind}.json', archive.namelist())
                for item in ('pattern', 'sky_slime_ball', 'ichor_slime_ball', 'ender_slime_ball', 'materials_and_you'):
                    self.assertIn(f'data/tconstruct/horizon/item/{item}.json', archive.namelist())


if __name__ == '__main__':
    unittest.main()
