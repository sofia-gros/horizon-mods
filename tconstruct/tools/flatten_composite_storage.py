"""Convert the source's fixed two-layer storage model to ordinary geometry.

This handles the source storage composite with an inset metal cube and a
translucent outer cube. Other Forge composite models are deliberately rejected.
"""
import copy

DIRECTIONS = ('down', 'up', 'north', 'south', 'west', 'east')


def flatten(source, inset):
    if source.get('loader') != 'forge:composite':
        raise ValueError('expected a Forge composite')
    children = source.get('children', {})
    if set(children) != {'metal', 'slime'}:
        raise ValueError('expected the metal and slime layers')
    metal, slime = children['metal'], children['slime']
    if (metal.get('parent') != 'tconstruct:block/template/cube_all_14'
            or metal.get('render_type') != 'minecraft:solid'
            or slime.get('parent') != 'block/cube_all'
            or slime.get('render_type') != 'minecraft:translucent'):
        raise ValueError('unsupported source layer geometry')
    metal_texture = metal['textures']['all']
    slime_texture = slime['textures']['all']
    elements = copy.deepcopy(inset['elements'])
    if len(elements) != 1 or elements[0].get('from') != [1, 1, 1] or elements[0].get('to') != [15, 15, 15]:
        raise ValueError('unexpected inset cube geometry')
    for face in elements[0]['faces'].values():
        if face['texture'] not in ('all', '#all'):
            raise ValueError('unexpected inset texture')
        face['texture'] = '#metal'
    elements.append({'from': [0, 0, 0], 'to': [16, 16, 16],
                     'faces': {d: {'texture': '#slime', 'cullface': d} for d in DIRECTIONS}})
    return {'parent': 'block/block',
            'textures': {'particle': source['textures']['particle'],
                         'metal': metal_texture, 'slime': slime_texture},
            'elements': elements}
