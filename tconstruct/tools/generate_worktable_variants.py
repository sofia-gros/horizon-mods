"""Generate source retextured worktables for the built-in source rock tags."""
import json
from pathlib import Path

root = Path(__file__).resolve().parents[1] / 'content'
assets = root / 'assets/tconstruct'
data = root / 'data/tconstruct'
rocks = ['stone', 'cobblestone', 'mossy_cobblestone', 'blackstone', 'granite', 'diorite', 'andesite', 'deepslate', 'cobbled_deepslate', 'basalt', 'tuff', 'dripstone_block', 'calcite']
for rock in rocks:
    name = 'modifier_worktable_' + rock
    side = 'minecraft:block/' + ('basalt_side' if rock == 'basalt' else rock)
    top = 'minecraft:block/' + ({'basalt': 'basalt_top', 'deepslate': 'deepslate_top'}.get(rock, rock))
    variants = {
        f'models/block/table/{name}.json': {'parent': 'tconstruct:block/table/modifier_worktable', 'textures': {'leg': side, 'legBottom': top}},
        f'models/item/{name}.json': {'parent': 'tconstruct:block/table/' + name},
        f'items/{name}.json': {'model': {'type': 'minecraft:model', 'model': 'tconstruct:item/' + name}},
    }
    for path, content in variants.items():
        dest = assets / path
        dest.parent.mkdir(parents=True, exist_ok=True)
        dest.write_text(json.dumps(content, separators=(',', ':')) + '\n')
    recipe = {'type': 'minecraft:crafting_shaped', 'category': 'misc', 'pattern': ['sss', 'r r', 'r r'],
              'key': {'s': '#tconstruct:anvil_seared_blocks', 'r': 'minecraft:' + rock},
              'result': {'id': 'tconstruct:modifier_worktable', 'components': {
                  'minecraft:custom_data': {'tconstruct:anvil': {'Rock': 'minecraft:' + rock, 'Variant': rock, 'Texture': side, 'TextureTop': top}},
                  'minecraft:item_model': 'tconstruct:' + name}}}
    dest = data / 'recipe' / (name + '.json')
    dest.write_text(json.dumps(recipe, indent=2) + '\n')
