"""Fill the TConstruct block group from its actual block declarations."""
import copy
import json


def add_block_items(groups, block_directory):
    result = copy.deepcopy(groups)
    group = next(group for group in result if group['id'] == 'tconstruct:blocks')
    if group['items']:
        raise ValueError('block group must be populated from block declarations')
    for path in sorted(block_directory.glob('*.json')):
        declaration = json.loads(path.read_text())
        if declaration.get('item', True):
            group['items'].append(f'{{count:1,id:"tconstruct:{path.stem}"}}')
    # Seeds belong to the world/blocks tab upstream but are standalone items.
    items = block_directory.parent / 'item'
    for kind in ('earth', 'sky', 'ender', 'blood', 'ichor'):
        name = f'{kind}_slime_grass_seeds'
        if (items / (name + '.json')).is_file():
            group['items'].append(f'{{count:1,id:"tconstruct:{name}"}}')
    return result
