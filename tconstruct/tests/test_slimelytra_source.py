import json
import unittest
from pathlib import Path
from zipfile import ZipFile

ROOT = Path(__file__).resolve().parents[1]

class SlimelytraSourceTests(unittest.TestCase):
    def test_pinned_recipe_and_material_data(self):
        with ZipFile(ROOT / 'content/reference/tconstruct-recipes.zip') as archive:
            recipe = json.loads(archive.read('data/tconstruct/recipes/tools/armor/slimelytra.json'))
            fluid = json.loads(archive.read('data/tconstruct/recipes/tools/materials/casting/earthslime.json'))
        self.assertEqual(recipe, {'type': 'tconstruct:basin_tool_casting', 'cast': {'item': 'minecraft:elytra'}, 'cast_purpose': 'consumed', 'item_cost': 8, 'result': 'tconstruct:slime_wings'})
        self.assertEqual(fluid['fluid']['amount'] * recipe['item_cost'], 2000)
        self.assertEqual(fluid['output'], 'tconstruct:earthslime')
        with ZipFile(ROOT / 'content/reference/tconstruct-tool-definitions.zip') as archive:
            definition = json.loads(archive.read('data/tconstruct/tinkering/tool_definitions/slime_wings.json'))
        modules = definition['modules']
        self.assertTrue(any(m.get('type') == 'tconstruct:material_stats' and m['stat_types'][0]['stat'] == 'tconstruct:slime' for m in modules))
        self.assertTrue(any(t['name'] == 'tconstruct:wings' for m in modules if m.get('type') == 'tconstruct:traits' for t in m['traits']))
        with ZipFile(ROOT / 'content/reference/tconstruct-materials.zip') as archive:
            traits = json.loads(archive.read('data/tconstruct/tinkering/materials/traits/earthslime.json'))
        self.assertEqual([t['name'] for t in traits['perStat']['tconstruct:slime']], ['tconstruct:overgrowth', 'tconstruct:overslime'])

if __name__ == '__main__': unittest.main()
