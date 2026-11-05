import json
import unittest
from pathlib import Path
from zipfile import ZipFile

ROOT = Path(__file__).resolve().parents[1]

class SlimelytraSourceTests(unittest.TestCase):
    def test_damaged_input_runtime_keeps_trait_and_registry_checks(self):
        source=(ROOT/'mixins/tconstruct/source-melter-world.rs').read_text()
        body=source.split('// Build a fresh slimelytra',1)[1].split('// Clearing an already-tipped',1)[0]
        self.assertIn('if basin && super::tconstruct_source_melter_state::consumed_elytra_with_presentation(stack)',body)
        self.assertNotIn('stack.damage == 0',body)
        self.assertIn('native.contains_key("tconstruct:slime_wings")',body)
        self.assertIn('levels_from_definition',body)
        self.assertIn('"tconstruct:slime_wings", &identities, None, &mut None',body)
        self.assertIn('levels(world, &output)?',body)
        runner=(ROOT/'tests/run_seared_reinforcement_rust.sh').read_text()
        self.assertIn('worn_elytra_recipe_keeps_input_until_full_valid_cooling_and_outputs_fresh_wings',runner)
    def test_damage_fixture_matches_pinned_archive(self):
        with ZipFile(ROOT/'content/reference/tconstruct-recipes.zip') as archive:
            for path in ['tools/armor/slimelytra','tools/materials/casting/earthslime']:
                fixture=ROOT/'tests/fixtures'/f"{path.rsplit('/',1)[-1]}_damaged_casting.json"
                self.assertEqual(json.loads(fixture.read_bytes()),json.loads(archive.read(f'data/tconstruct/recipes/{path}.json')))
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
