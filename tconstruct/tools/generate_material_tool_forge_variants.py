"""Project the two source material Tool Forge recipes onto six native alloys."""
import copy
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
DATA = ROOT / 'content/data/tconstruct'
SOURCE = ROOT / 'tests/fixtures/material_tool_forge_acquisition'
METALS = ('bronze', 'constantan', 'invar', 'electrum', 'pewter', 'nicrosil')


def generate():
    for source_id, forge, anvil in [('seared_forge_material', 'tinkers_forge', 'tinkers_anvil'),
                                     ('scorched_forge_material', 'scorched_forge', 'scorched_anvil')]:
        source = json.loads((SOURCE / 'recipes' / (source_id + '.json')).read_text())
        recipe_ids = []
        for metal in METALS:
            base = json.loads((DATA / 'recipe' / f'{anvil}_compat_{metal}.json').read_text())
            result = copy.deepcopy(base['result'])
            result['components']['minecraft:custom_name'] = json.loads(source['result']['nbt']['display']['Name'])
            recipe = {'type': 'minecraft:crafting_shaped', 'category': source['category'],
                      'pattern': source['pattern'], 'key': {
                          'm': source['key']['m']['item'], 's': '#' + source['key']['s']['tag'],
                          't': source['key']['t']['item']},
                      'result': result, 'show_notification': source['show_notification']}
            name = f'{forge}_compat_{metal}'
            (DATA / 'recipe' / (name + '.json')).write_text(json.dumps(recipe, indent=2) + '\n')
            recipe_ids.append((metal, 'tconstruct:' + name))
        goal = json.loads((SOURCE / 'advancements' / (source_id + '.json')).read_text())
        del goal['criteria']['has_the_recipe']
        criteria = ['has_item']
        for metal, recipe_id in recipe_ids:
            criterion = 'has_recipe_' + metal
            goal['criteria'][criterion] = {'trigger': 'minecraft:recipe_unlocked', 'conditions': {'recipes': recipe_id}}
            criteria.append(criterion)
        goal['requirements'] = [criteria]
        goal['rewards']['recipes'] = [recipe_id for _, recipe_id in recipe_ids]
        (DATA / 'advancement/recipes/decorations' / (source_id + '.json')).write_text(json.dumps(goal, indent=2) + '\n')


if __name__ == '__main__':
    generate()
