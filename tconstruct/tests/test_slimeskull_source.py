import json
import unittest
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]

class SlimeskullSourceTest(unittest.TestCase):
    def test_fixed_recipe_and_material_order(self):
        with zipfile.ZipFile(ROOT / 'content/reference/tconstruct-recipes.zip') as source:
            recipe = json.loads(source.read('data/tconstruct/recipes/tools/armor/slime_skull/bone.json'))
            fluid = json.loads(source.read('data/tconstruct/recipes/tools/materials/casting/earthslime.json'))
        self.assertEqual(recipe['cast'], {'item': 'minecraft:skeleton_skull'})
        self.assertEqual((recipe['cast_purpose'], recipe['extra_materials'], recipe['item_cost'], recipe['fluid_swapping'], recipe['result']),
                         ('consumed_offset', ['tconstruct:bone'], 5, False, 'tconstruct:slime_helmet'))
        self.assertEqual((fluid['output'], fluid['fluid']['amount'] * recipe['item_cost']), ('tconstruct:earthslime', 1250))
        with zipfile.ZipFile(ROOT / 'content/reference/tconstruct-tool-definitions.zip') as source:
            definition = json.loads(source.read('data/tconstruct/tinkering/tool_definitions/slime_helmet.json'))
        module = next(m for m in definition['modules'] if m['type'] == 'tconstruct:material_stats')
        self.assertEqual(module['stat_types'][0], 'tconstruct:skull')
        self.assertEqual(module['stat_types'][1]['stat'], 'tconstruct:slime')

    def test_world_path_fails_closed_on_data_and_resolves_traits(self):
        source = (ROOT / 'mixins/tconstruct/source-melter-world.rs').read_text()
        self.assertIn('if basin && super::tconstruct_source_melter_state::consumed_head_with_presentation(stack)', source)
        self.assertIn('catalog.slimeskull_casting(true, &stack.item, fluid', source)
        self.assertIn('let identities = vec![skull, slime];', source)
        self.assertIn('tconstruct_source_material_traits::levels(world, &output)?', source)
        self.assertIn('item_from_nbt(&horizon_items::item_nbt::item_to_nbt(&output))?', source)

if __name__ == '__main__':
    unittest.main()
