"""Source survival chain ingredients, output counts and heat timings."""
import json
from pathlib import Path
import unittest
from zipfile import ZipFile

ROOT = Path(__file__).resolve().parents[1]
DATA = ROOT / 'content/data/tconstruct'

class SurvivalSmelteryRecipes(unittest.TestCase):
    def test_source_recipes_keep_shapes_outputs_and_cooking(self):
        examples = {
            'seared': ['grout', 'grout_multiple', 'seared_brick', 'seared_bricks_from_brick', 'heater', 'fuel_gauge', 'fuel_tank', 'melter', 'faucet', 'basin', 'table', 'drain'],
            'scorched': ['nether_grout', 'nether_grout_multiple', 'scorched_brick', 'scorched_bricks_from_brick', 'fuel_gauge', 'fuel_tank', 'drain'],
        }
        with ZipFile(ROOT / 'content/reference/tconstruct-recipes.zip') as archive:
            for group, names in examples.items():
                for name in names:
                    with self.subTest(group=group, name=name):
                        original = json.loads(archive.read(f'data/tconstruct/recipes/smeltery/{group}/{name}.json'))
                        adapted = json.loads((DATA / f'recipe/survival_{group}_{name}.json').read_text())
                        self.assertEqual(adapted['type'], original['type'])
                        self.assertEqual(adapted.get('pattern'), original.get('pattern'))
                        self.assertEqual(adapted.get('cookingtime'), original.get('cookingtime'))
                        self.assertEqual(adapted.get('experience'), original.get('experience'))
                        source = original['result']; source = {'id': source} if isinstance(source, str) else {'id': source['item'], **({'count': source['count']} if source.get('count', 1) != 1 else {})}
                        self.assertEqual(adapted['result'], source)
        gauge = json.loads((DATA / 'recipe/survival_seared_melter.json').read_text())
        self.assertEqual(set(gauge['key']['G']), {'tconstruct:seared_fuel_gauge', 'tconstruct:seared_ingot_gauge'})
        self.assertEqual(gauge['key']['B'], 'tconstruct:seared_brick')

    def test_glass_tag_reuses_vanilla_and_tconstruct_source(self):
        from test_survival_reachability import tag_items
        values = json.loads((DATA / 'tags/item/source_forge_glass.json').read_text())['values']
        self.assertEqual(values, ['#forge:glass'])
        with ZipFile(ROOT / 'content/reference/forge-47.4.0-data.zip') as forge:
            stained = json.loads(forge.read('data/forge/tags/items/stained_glass.json'))['values']
        source = ROOT / 'content/reference/tconstruct-tags/data/forge/tags/items/stained_glass.json'
        clear_stained = json.loads(source.read_text())['values']
        self.assertEqual(tag_items('tconstruct:source_forge_glass'), {
            'minecraft:glass', 'minecraft:tinted_glass', 'tconstruct:soul_glass',
            'tconstruct:clear_glass', 'tconstruct:clear_tinted_glass', *stained, *clear_stained})

if __name__ == '__main__':
    unittest.main()
