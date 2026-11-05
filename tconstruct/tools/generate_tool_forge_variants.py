"""Project pinned Tool Forge recipes onto the existing ordinary Anvil variants."""
import copy
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
DATA = ROOT / 'content/data/tconstruct'
SOURCE = ROOT / 'tests/fixtures/tool_forge_acquisition'
METALS = ('slimesteel', 'amethyst_bronze', 'rose_gold', 'pig_iron', 'cinderslime',
          'queens_slime', 'manyullyn', 'hepatizon', 'knightmetal', 'knightslime', 'netherite')


def generate():
    for forge, anvil in [('tinkers_forge', 'tinkers_anvil'), ('scorched_forge', 'scorched_anvil')]:
        source = json.loads((SOURCE / 'recipes' / (forge + '.json')).read_text())
        name = json.loads(source['result']['nbt']['display']['Name'])
        recipes = []
        for metal in (*METALS, 'mixed_alloys'):
            prefix = 'zz_' if metal == 'mixed_alloys' else ''
            base = json.loads((DATA / 'recipe' / f'{prefix}{anvil}_{metal}.json').read_text())
            recipe = {'type': 'minecraft:crafting_shaped', 'category': source['category'],
                      'pattern': source['pattern'],
                      'key': {'s': '#' + source['key']['s']['tag'], 'm': base['key']['m'],
                              't': source['key']['t']['item']},
                      'result': copy.deepcopy(base['result']),
                      'show_notification': source['show_notification']}
            recipe['result'].setdefault('components', {})['minecraft:custom_name'] = name
            recipe_id = f'{prefix}{forge}_{metal}'
            (DATA / 'recipe' / (recipe_id + '.json')).write_text(json.dumps(recipe, indent=2) + '\n')
            recipes.append((metal, 'tconstruct:' + recipe_id))

        goal = json.loads((SOURCE / 'advancements' / (forge + '.json')).read_text())
        item = goal['criteria']['has_item']['conditions']['items'][0]
        item['items'] = '#' + item.pop('tag')
        del goal['criteria']['has_the_recipe']
        criteria = ['has_item']
        for metal, recipe_id in recipes:
            criterion = 'has_recipe_' + metal
            goal['criteria'][criterion] = {'trigger': 'minecraft:recipe_unlocked',
                                          'conditions': {'recipes': recipe_id}}
            criteria.append(criterion)
        goal['requirements'] = [criteria]
        goal['rewards']['recipes'] = [recipe_id for _, recipe_id in recipes]
        target = DATA / 'advancement/recipes/decorations' / (forge + '.json')
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(json.dumps(goal, indent=2) + '\n')


if __name__ == '__main__':
    generate()
