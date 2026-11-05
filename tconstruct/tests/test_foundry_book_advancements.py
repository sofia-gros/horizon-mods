"""Source Foundry entry progression and its native recipe-book unlock."""
import json
import sys
import tempfile
import unittest
import zipfile
from pathlib import Path

MOD = Path(__file__).resolve().parents[1]
DATA = MOD / 'content/data/tconstruct'
sys.path.insert(0, str(MOD / 'tools'))
from package_hmod import package


def read(path):
    return json.loads((DATA / path).read_text())


class FoundryBookAdvancements(unittest.TestCase):
    def test_source_acquisition_and_existing_parent(self):
        value = read('advancement/foundry/fantastic_foundry.json')
        self.assertEqual(value['parent'], 'tconstruct:tools/materials_and_you')
        self.assertTrue((DATA / 'advancement/tools/materials_and_you.json').is_file())
        self.assertEqual(value['criteria'], {'crafted_book': {
            'trigger': 'minecraft:inventory_changed',
            'conditions': {'items': [{'items': ['tconstruct:fantastic_foundry']}]},
        }})
        self.assertEqual(value['requirements'], [['crafted_book']])
        self.assertEqual(value['display'], {
            'announce_to_chat': False,
            'description': {'translate': 'advancements.tconstruct.foundry.fantastic_foundry.description'},
            'frame': 'task', 'hidden': False,
            'icon': {'id': 'tconstruct:fantastic_foundry'},
            'show_toast': True,
            'title': {'translate': 'advancements.tconstruct.foundry.fantastic_foundry.title'},
        })
        self.assertFalse(value['sends_telemetry_event'])
        self.assertNotIn('rewards', value)

    def test_source_unlock_conditions_adapted_to_native_recipe(self):
        value = read('advancement/recipes/misc/fantastic_foundry.json')
        self.assertEqual(value, {
            'parent': 'minecraft:recipes/root',
            'criteria': {
                'has_item': {
                    'trigger': 'minecraft:inventory_changed',
                    'conditions': {'items': [{'items': ['tconstruct:nether_grout']}]},
                },
                'has_the_recipe': {
                    'trigger': 'minecraft:recipe_unlocked',
                    'conditions': {'recipes': 'tconstruct:fantastic_foundry'},
                },
            },
            'requirements': [['has_item', 'has_the_recipe']],
            'rewards': {'recipes': ['tconstruct:fantastic_foundry']},
            'sends_telemetry_event': False,
        })
        recipe = read('recipe/fantastic_foundry.json')
        self.assertEqual(recipe['type'], 'minecraft:crafting_shapeless')
        self.assertCountEqual(recipe['ingredients'], ['minecraft:book', 'tconstruct:nether_grout'])
        self.assertEqual(recipe['result'], {'id': 'tconstruct:fantastic_foundry', 'count': 1})
        self.assertTrue((DATA / 'horizon/item/fantastic_foundry.json').is_file())
        self.assertTrue((DATA / 'horizon/block/nether_grout.json').is_file())

    def test_package_retains_both_advancements_recipe_and_translations(self):
        with tempfile.TemporaryDirectory() as tmp:
            output = Path(tmp) / 'tconstruct.hmod'
            package(output)
            with zipfile.ZipFile(output) as packed:
                for path in ('advancement/foundry/fantastic_foundry.json',
                             'advancement/recipes/misc/fantastic_foundry.json',
                             'recipe/fantastic_foundry.json'):
                    self.assertEqual(json.loads(packed.read('data/tconstruct/' + path)), read(path))
                language = json.loads(packed.read('assets/tconstruct/lang/en_us.json'))
                self.assertEqual(json.loads(packed.read('data/tconstruct/horizon/block/nether_grout.json')),
                                 read('horizon/block/nether_grout.json'))
                for key in ('title', 'description'):
                    self.assertIn('advancements.tconstruct.foundry.fantastic_foundry.' + key, language)


if __name__ == '__main__':
    unittest.main()
