"""Pinned water bottle casting inputs and authoritative stack construction."""
import json
from pathlib import Path
import unittest
from zipfile import ZipFile

ROOT = Path(__file__).resolve().parents[1]

class WaterBottleCasting(unittest.TestCase):
    def test_source_recipes_are_pinned(self):
        with ZipFile(ROOT / 'content/reference/tconstruct-recipes.zip') as archive:
            for kind, item in (('bottle', 'potion'), ('splash', 'splash_potion'), ('lingering', 'lingering_potion')):
                recipe = json.loads(archive.read(f'data/tconstruct/recipes/smeltery/casting/water/{kind}.json'))
                self.assertEqual(recipe, json.loads((ROOT / f'tests/fixtures/water_casting_{kind}.json').read_text()))
                self.assertEqual(recipe['result'], {'item': 'minecraft:' + item, 'nbt': {'Potion': 'minecraft:water'}})
                self.assertEqual(recipe['fluid'], {'amount': 500, 'tag': 'mantle:water'})
                self.assertEqual(recipe['cooling_time'], 1)
                self.assertTrue(recipe['cast_consumed'])

    def test_normal_casting_preserves_result_potion_before_prepare(self):
        world = (ROOT / 'mixins/tconstruct/source-melter-world.rs').read_text()
        begin = world.index('    if let Some((id, recipe, output)) = casting(world, basin, input_id, fluid) {')
        end = world.index('    if let Some(stack) = input {', begin)
        route = world[begin:end]
        self.assertIn('if let Some(potion) = recipe.output.potion() { stack = stack.with_potion(potion); }', route)
        self.assertIn('return Some((id, recipe, stack));', route)
