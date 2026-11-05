"""Source progression into Grout, smelted bricks, and the Puny guide."""
import json
import sys
import tempfile
import unittest
import zipfile
from pathlib import Path

MOD = Path(__file__).resolve().parents[1]
DATA = MOD / 'content/data/tconstruct'
# native recipe: (advancement category, source recipe path, source unlock item)
CASES = {
    'survival_seared_grout': ('building_blocks', 'smeltery/seared/grout', 'minecraft:clay_ball'),
    'survival_seared_grout_multiple': ('building_blocks', 'smeltery/seared/grout_multiple', 'minecraft:clay'),
    'survival_seared_seared_brick': ('building_blocks', 'smeltery/seared/seared_brick', 'tconstruct:grout'),
    'survival_scorched_nether_grout': ('building_blocks', 'smeltery/scorched/nether_grout', 'minecraft:magma_cream'),
    'survival_scorched_nether_grout_multiple': ('building_blocks', 'smeltery/scorched/nether_grout_multiple', 'minecraft:magma_cream'),
    'survival_scorched_scorched_brick': ('building_blocks', 'smeltery/scorched/scorched_brick', 'tconstruct:nether_grout'),
    'puny_smelting': ('misc', 'common/puny_smelting', 'tconstruct:grout'),
}
sys.path.insert(0, str(MOD / 'tools'))
from package_hmod import package


def read(path):
    return json.loads((DATA / path).read_text())


class GroutRecipeUnlocks(unittest.TestCase):
    def test_complete_source_predicates_and_native_reward_mapping(self):
        for name, (category, _, item) in CASES.items():
            with self.subTest(recipe=name):
                native_id = 'tconstruct:' + name
                self.assertEqual(read(f'advancement/recipes/{category}/{name}.json'), {
                    'parent': 'minecraft:recipes/root',
                    'criteria': {
                        'has_item': {
                            'trigger': 'minecraft:inventory_changed',
                            'conditions': {'items': [{'items': [item]}]},
                        },
                        'has_the_recipe': {
                            'trigger': 'minecraft:recipe_unlocked',
                            'conditions': {'recipes': native_id},
                        },
                    },
                    'requirements': [['has_item', 'has_the_recipe']],
                    'rewards': {'recipes': [native_id]},
                    'sends_telemetry_event': False,
                })

    def test_rewarded_recipes_retain_source_type_yield_and_smelting(self):
        with zipfile.ZipFile(MOD / 'content/reference/tconstruct-recipes.zip') as source:
            for name, (_, path, _) in CASES.items():
                with self.subTest(recipe=name):
                    original = json.loads(source.read(f'data/tconstruct/recipes/{path}.json'))
                    native = read(f'recipe/{name}.json')
                    self.assertEqual(native['type'], original['type'])
                    result = original['result']
                    if isinstance(result, str):
                        result = {'item': result}
                    self.assertEqual(native['result']['id'], result['item'])
                    self.assertEqual(native['result'].get('count', 1), result.get('count', 1))
                    for key in ('experience', 'cookingtime'):
                        self.assertEqual(native.get(key), original.get(key))
        # Source intentionally unlocks the bulk Nether recipe on Magma Cream,
        # though crafting it uses a Magma Block. Do not infer unlocks from inputs.
        self.assertEqual(CASES['survival_scorched_nether_grout_multiple'][2], 'minecraft:magma_cream')
        self.assertEqual(read('recipe/survival_scorched_nether_grout_multiple.json')['ingredients'][0],
                         'minecraft:magma_block')

    def test_package_contains_complete_chain_and_target_definitions(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'tconstruct.hmod'
            package(path)
            with zipfile.ZipFile(path) as archive:
                for name, (category, _, _) in CASES.items():
                    for relative in (f'advancement/recipes/{category}/{name}.json', f'recipe/{name}.json'):
                        self.assertEqual(json.loads(archive.read('data/tconstruct/' + relative)), read(relative))
                for block in ('grout', 'nether_grout'):
                    declaration = json.loads(archive.read(f'data/tconstruct/horizon/block/{block}.json'))
                    self.assertTrue(declaration.get('item', True))
                for item in ('seared_brick', 'scorched_brick', 'puny_smelting'):
                    self.assertIsInstance(json.loads(archive.read(f'data/tconstruct/horizon/item/{item}.json')), dict)
                # This recipe unlock supplements, not replaces, acquiring the book.
                book = json.loads(archive.read('data/tconstruct/advancement/smeltery/puny_smelting.json'))
                self.assertEqual(book['criteria']['crafted_book']['conditions']['items'][0]['items'],
                                 ['tconstruct:puny_smelting'])


if __name__ == '__main__':
    unittest.main()
