import json
import unittest
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]

class FirstMaterialSourceTest(unittest.TestCase):
    def test_fixed_recipes_and_definition_order(self):
        cases = [
            ('tools/armor/plate/plate_shield', 'table', 'shield_core', 'plate_shield', 3, 'plating_shield'),
            ('tools/building/slime_boots', 'basin', 'laces', 'slime_boots', 4, 'slime'),
            ('tools/building/slimecage', 'basin', 'ribcage', 'slimy_chestplate', 8, 'slime'),
            ('tools/building/slimeshell', 'basin', 'shell', 'slime_leggings', 7, 'slime'),
        ]
        with zipfile.ZipFile(ROOT / 'content/reference/tconstruct-recipes.zip') as recipes, zipfile.ZipFile(ROOT / 'content/reference/tconstruct-tool-definitions.zip') as defs:
            for path, kind, part, result, cost, fluid_stat in cases:
                with self.subTest(result=result):
                    r = json.loads(recipes.read('data/tconstruct/recipes/' + path + '.json'))
                    self.assertEqual(r, {'type': f'tconstruct:{kind}_tool_casting', 'cast': {'type': 'tconstruct:material', 'item': f'tconstruct:{part}'}, 'cast_purpose': 'first_material', 'item_cost': cost, 'result': f'tconstruct:{result}'})
                    name = 'slime_chestplate' if result == 'slimy_chestplate' else result
                    d = json.loads(defs.read(f'data/tconstruct/tinkering/tool_definitions/{name}.json'))
                    stats = next(m for m in d['modules'] if m['type'] == 'tconstruct:material_stats')['stat_types']
                    self.assertEqual(stats[0], f'tconstruct:{part}')
                    self.assertEqual(stats[1] if isinstance(stats[1], str) else stats[1]['stat'], f'tconstruct:{fluid_stat}')

    def test_runtime_uses_guarded_part_and_preserves_order(self):
        s = (ROOT / 'mixins/tconstruct/source-melter-world.rs').read_text()
        self.assertIn('plain_material_part(stack)', s)
        self.assertIn('let identities = vec![first, fluid_material];', s)
        self.assertIn('supported_material(world, &first)', s)
        self.assertIn('item_from_nbt(&horizon_items::item_nbt::item_to_nbt(&output))?', s)

if __name__ == '__main__':
    unittest.main()
