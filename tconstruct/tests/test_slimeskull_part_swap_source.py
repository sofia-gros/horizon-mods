import json
import unittest
import zipfile
from pathlib import Path
ROOT = Path(__file__).resolve().parents[1]

class SlimeskullPartSwapTest(unittest.TestCase):
    def test_fixed_source_recipe_and_definition(self):
        with zipfile.ZipFile(ROOT / 'content/reference/tconstruct-recipes.zip') as recipes:
            recipe = json.loads(recipes.read('data/tconstruct/recipes/tools/armor/slime_skull/swapping/slime.json'))
        self.assertEqual(recipe, {'type': 'tconstruct:basin_casting_part_swapping', 'index': 1, 'item_cost': 5, 'tools': {'item': 'tconstruct:slime_helmet'}})
        with zipfile.ZipFile(ROOT / 'content/reference/tconstruct-tool-definitions.zip') as defs:
            definition = json.loads(defs.read('data/tconstruct/tinkering/tool_definitions/slime_helmet.json'))
        stats = next(m for m in definition['modules'] if m['type'] == 'tconstruct:material_stats')['stat_types']
        self.assertEqual(stats[0], 'tconstruct:skull')
        self.assertEqual(stats[1]['stat'], 'tconstruct:slime')

    def test_exclusive_guard_and_safe_rebuild_wiring(self):
        world = (ROOT / 'mixins/tconstruct/source-melter-world.rs').read_text()
        self.assertLess(world.index('catalog.has_slimeskull_swap'), world.index('if let Some((id, recipe, output)) = casting(world, basin, input_id, fluid)'))
        self.assertIn('stack.same_item_same_components(&pristine)', world)
        self.assertIn('identities[1] = material;', world)
        self.assertIn('rebuild_material_swap(world, definition, stack, &identities, 0, true', world)
        self.assertIn('item_from_nbt(&horizon_items::item_nbt::item_to_nbt(&output))?', world)

if __name__ == '__main__':
    unittest.main()
