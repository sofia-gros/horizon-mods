import json
import unittest
import zipfile
from pathlib import Path
ROOT = Path(__file__).resolve().parents[1]

class ToolFluidSwapSourceTest(unittest.TestCase):
    def test_source_defaults_and_slimeskull_opt_out(self):
        with zipfile.ZipFile(ROOT / 'content/reference/tconstruct-recipes.zip') as source:
            for path in ['tools/armor/slimelytra', 'tools/armor/plate/plate_shield', 'tools/building/slime_boots', 'tools/building/slimecage', 'tools/building/slimeshell']:
                recipe = json.loads(source.read('data/tconstruct/recipes/' + path + '.json'))
                self.assertNotIn('fluid_swapping', recipe)
            skull = json.loads(source.read('data/tconstruct/recipes/tools/armor/slime_skull/bone.json'))
            self.assertIs(skull['fluid_swapping'], False)

    def test_world_guards_and_last_material_rebuild(self):
        world = (ROOT / 'mixins/tconstruct/source-melter-world.rs').read_text()
        self.assertLess(world.index('catalog.tool_swap_overlap(basin, item'), world.index('if let Some((id, recipe, output)) = casting(world, basin, input_id, fluid)'))
        self.assertLess(world.index('catalog.tool_swap_overlap(basin, item'), world.index('catalog.part_swap_casting(basin'))
        self.assertIn('stack.same_item_same_components(&pristine)', world)
        self.assertIn('*identities.last_mut()? = material;', world)
        self.assertIn('rebuild_material_swap(world, definition, stack, &identities, 0, true', world)
        self.assertIn('item_from_nbt(&horizon_items::item_nbt::item_to_nbt(&output))?', world)

if __name__ == '__main__':
    unittest.main()
