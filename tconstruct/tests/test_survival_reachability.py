"""Data-only early progression checks; not proof of fixture-free gameplay.

Vanilla gathering/crafting is an explicit boundary. No TConstruct item is
seeded. Reachability ignores quantities; source volume checks cover the alloy
route separately. Runtime menus, fuel use and natural generation need playtests.
"""
import json
from pathlib import Path
import unittest
from zipfile import ZipFile

ROOT = Path(__file__).resolve().parents[1]
DATA = ROOT / 'content/data/tconstruct'
# Naturally gathered items or ordinary vanilla crafting/smelting outputs.
VANILLA = {'minecraft:' + name for name in (
    'oak_planks', 'stick', 'crafting_table', 'clay_ball', 'sand', 'gravel',
    'glass', 'copper_ingot', 'gold_ingot', 'coal', 'flint', 'oak_log')}
# Only vanilla tags needed by this route; other tags must resolve from data.
VANILLA_TAGS = {'minecraft:planks': ['minecraft:oak_planks'],
                'minecraft:sand': ['minecraft:sand']}
EARLY = {'tconstruct:' + name for name in (
    'pattern', 'part_builder', 'tinker_station', 'grout', 'seared_brick',
    'seared_bricks', 'seared_heater', 'seared_fuel_gauge', 'seared_melter',
    'seared_faucet', 'seared_table', 'seared_basin', 'seared_fuel_tank',
    'seared_drain', 'blank_sand_cast')}


def read(path):
    return json.loads(path.read_text())


def tag_items(tag, visiting=()):
    if tag in visiting:
        raise AssertionError('cyclic item tag: ' + ' -> '.join((*visiting, tag)))
    if tag in VANILLA_TAGS:
        return set(VANILLA_TAGS[tag])
    namespace, name = tag.split(':')
    path = ROOT / f'content/data/{namespace}/tags/item/{name}.json'
    if not path.is_file():
        return set()  # Unknown inputs never become freely obtainable.
    result = set()
    for value in read(path)['values']:
        value = value['id'] if isinstance(value, dict) else value
        result.update(tag_items(value[1:], (*visiting, tag)) if value.startswith('#') else [value])
    return result


def matches(ingredient, reachable):
    if isinstance(ingredient, list):
        return any(matches(choice, reachable) for choice in ingredient)
    if not isinstance(ingredient, str):
        raise AssertionError(f'unhandled native ingredient: {ingredient!r}')
    return bool(tag_items(ingredient[1:]) & reachable) if ingredient.startswith('#') else ingredient in reachable


def closure(recipes):
    reachable = set(VANILLA)
    while True:
        previous = set(reachable)
        for recipe in recipes:
            kind = recipe['type']
            if kind == 'minecraft:crafting_shaped':
                inputs = [recipe['key'][c] for row in recipe['pattern'] for c in row if c != ' ']
            elif kind == 'minecraft:crafting_shapeless':
                inputs = recipe['ingredients']
            elif kind == 'minecraft:smelting':
                inputs = [recipe['ingredient']]
            else:
                continue
            if all(matches(item, reachable) for item in inputs):
                reachable.add(recipe['result']['id'])
        if previous == reachable:
            return reachable


class SurvivalReachability(unittest.TestCase):
    def setUp(self):
        self.recipes = [read(p) for p in sorted((DATA / 'recipe').glob('*.json'))]

    def test_early_devices_have_no_mod_item_seed_or_dependency_cycle(self):
        reachable = closure(self.recipes)
        self.assertFalse(EARLY - reachable, f'unreachable: {sorted(EARLY - reachable)}')
        # An alloy block is not available just because its block/ingot recipes
        # refer to one another. A real melting/alloy/casting step is required.
        self.assertNotIn('tconstruct:rose_gold_block', reachable)
        self.assertNotIn('tconstruct:tinkers_anvil', reachable)
        # Machine/block declarations are shipped directly; loose items and
        # casts are registered by resources.py and tested separately.
        for item in EARLY - {'tconstruct:pattern', 'tconstruct:seared_brick', 'tconstruct:blank_sand_cast'}:
            name = item.split(':')[1]
            self.assertTrue(any((DATA / f'horizon/{kind}/{name}.json').is_file()
                                for kind in ('item', 'block')), item)

    def test_removing_grout_breaks_the_machine_chain(self):
        recipes = [r for r in self.recipes if r['result']['id'] != 'tconstruct:grout']
        reachable = closure(recipes)
        self.assertIn('tconstruct:part_builder', reachable)
        self.assertNotIn('tconstruct:seared_melter', reachable)
        self.assertNotIn('tconstruct:seared_brick', reachable)

    def test_source_controller_and_alloy_route_quantities(self):
        with ZipFile(ROOT / 'content/reference/tconstruct-recipes.zip') as archive:
            self.assertEqual(len(archive.namelist()), len(set(archive.namelist())),
                             'duplicate recipe archive paths')
            def source(name):
                return json.loads(archive.read(f'data/tconstruct/recipes/{name}.json'))
            copper = source('smeltery/melting/metal/copper/ingot')
            gold = source('smeltery/melting/metal/gold/ingot')
            controller = source('smeltery/casting/seared/smeltery_controller')
            alloy = source('smeltery/alloys/molten_rose_gold')
            block = source('smeltery/casting/metal/rose_gold/block')
            self.assertEqual(controller['cast'], {'tag': 'tconstruct:smeltery_bricks'})
            self.assertTrue(controller['cast_consumed'])
            self.assertEqual(controller['fluid']['tag'], copper['result']['tag'])
            self.assertEqual(controller['fluid']['amount'], 4 * copper['result']['amount'])
            self.assertEqual(alloy['inputs'], [copper['result'], gold['result']])
            self.assertEqual(alloy['result']['tag'], block['fluid']['tag'])
            # Fourteen ingots of each metal give 2520 mB: three 810 mB
            # blocks and 90 mB left over. No pre-existing alloy is required.
            self.assertEqual(14 * alloy['result']['amount'], 3 * block['fluid']['amount'] + 90)
            self.assertNotIn('cast', block)
            anvil = read(DATA / 'recipe/tinkers_anvil_rose_gold.json')
            self.assertEqual(''.join(anvil['pattern']).count('m'), 3)
            self.assertEqual(anvil['key']['m'], 'tconstruct:rose_gold_block')
            self.assertIn('tconstruct:seared_bricks', tag_items(anvil['key']['s'][1:]))
            for name, cost in [('pick_head', 2), ('tool_handle', 1), ('tool_binding', 1)]:
                part = source(f'tools/parts/builder/{name}')
                self.assertEqual(part['cost'], cost)
                self.assertIn({'tag': 'tconstruct:patterns/default'}, part['pattern_item'])
                self.assertEqual(read(DATA / f'recipes/tools/parts/builder/{name}.json'), part)
            mold = source('smeltery/casts/sand/builder_cast/pick_head')
            self.assertEqual(mold['type'], 'tconstruct:item_part_builder')
            self.assertEqual(mold['pattern_item'], {'tag': 'tconstruct:casts/sand'})
            self.assertEqual(mold['result'], 'tconstruct:pick_head_sand_cast')
            copper_part = source('tools/materials/casting/copper')
            self.assertEqual(copper_part['fluid'], copper['result'])
            flint = source('tools/materials/flint/flint')
            self.assertEqual(flint['ingredient'], {'item': 'minecraft:flint'})
            self.assertEqual((flint['needed'], flint['value']), (1, 1))


if __name__ == '__main__':
    unittest.main()
