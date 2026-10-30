"""Generate ordinary Horizon models from the pinned connected-glass assets.

The generated geometry keeps the original textures. The runtime mesh hook
selects the per-neighbor variants; this file does not simulate connections.
"""
import json
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CONTENT = ROOT / 'content'
ARCHIVE = CONTENT / 'reference/tconstruct-connected-glass.zip'


def emit(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, separators=(',', ':')) + '\n')


def generate(root=CONTENT):
    with zipfile.ZipFile(ARCHIVE) as archive:
        source = {name.split('/assets/tconstruct/', 1)[1]: archive.read(name) for name in archive.namelist()}
    assets = root / 'assets/tconstruct'
    for name, data in source.items():
        if name.startswith('textures/'):
            path = assets / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(data)
    # Item tints operate on explicitly marked faces. Create copies of the
    # original pane geometry for colored glass; uncolored panes keep the source.
    for name, data in source.items():
        if not name.startswith('models/block/template/pane/'):
            continue
        model = json.loads(data)
        emit(assets / name, model)
        for element in model.get('elements', []):
            for face in element.get('faces', {}).values():
                face['tintindex'] = 0
        emit(assets / name.replace('template/pane/', 'template/tinted_pane/'), model)
    cube = {'parent': 'minecraft:block/block', 'textures': {'particle': '#all'},
            'elements': [{'from': [0, 0, 0], 'to': [16, 16, 16],
                          'faces': {d: {'texture': '#all', 'cullface': d, 'tintindex': 0}
                                    for d in ('down', 'up', 'north', 'south', 'west', 'east')}}]}
    emit(assets / 'models/block/template/tinted_connected_cube.json', cube)
    colors = {}
    for name, data in source.items():
        if not name.startswith('models/block/') or name.startswith('models/block/template/'):
            continue
        model = json.loads(data)
        color = model.pop('colors', None)
        model.pop('connection', None)
        loader = model.pop('loader', None)
        if loader and loader != 'mantle:connected':
            raise ValueError(f'unsupported glass loader {loader} in {name}')
        if color:
            if len(color) != 1 or set(color[0]) != {'color'}:
                raise ValueError(f'unsupported colors in {name}')
            colors[name.removeprefix('models/').removesuffix('.json')] = int(color[0]['color'], 16) & 0xffffff
            if model.get('parent') == 'minecraft:block/cube_all':
                model['parent'] = 'tconstruct:block/template/tinted_connected_cube'
            elif model.get('parent', '').startswith('tconstruct:block/template/pane/'):
                model['parent'] = model['parent'].replace('/template/pane/', '/template/tinted_pane/')
            else:
                raise ValueError(f'unsupported colored model parent in {name}')
        emit(assets / name, model)
    for name, data in source.items():
        if not name.startswith('blockstates/'):
            continue
        block = Path(name).stem
        # Soul glass has different collision and movement rules; it needs those
        # rules wired before it can safely be added to the playable inventory.
        if 'soul_glass' in block:
            continue
        emit(assets / name, json.loads(data))
        item_name = f'models/item/{block}.json'
        item = json.loads(source[item_name])
        layers = item.pop('layers', None)
        loader = item.pop('loader', None)
        if loader and loader != 'mantle:item_layer':
            raise ValueError(f'unsupported item loader {loader}')
        emit(assets / item_name, item)
        leaf = {'type': 'minecraft:model', 'model': f'tconstruct:item/{block}'}
        color = None
        if layers:
            color = int(layers[0]['color'], 16) & 0xffffff
        elif item.get('parent', '').startswith('tconstruct:'):
            color = colors.get(item['parent'].removeprefix('tconstruct:'))
        if color is not None:
            leaf['tints'] = [{'type': 'minecraft:constant', 'value': color}]
        emit(assets / f'items/{block}.json', {'model': leaf})
        pane = block.endswith('_pane')
        structural = block.startswith(('seared_', 'scorched_'))
        properties = {'hardness': 3.0 if block.startswith('seared_') else 2.5 if structural else 0.3,
                      'explosion_resistance': 9.0 if block.startswith('seared_') else 8.0 if structural else 0.3,
                      'requires_tool': structural, 'sounds': 'minecraft:glass'}
        declaration = {'template': 'minecraft:glass_pane' if pane else 'minecraft:glass', 'properties': properties}
        emit(root / f'data/tconstruct/horizon/block/{block}.json', declaration)
        emit(root / f'data/tconstruct/loot_table/blocks/{block}.json', {
            'type': 'minecraft:block', 'pools': [{'condition': {'type': 'minecraft:survives_explosion'},
            'entries': [{'type': 'minecraft:item', 'name': f'tconstruct:{block}'}], 'rolls': 1}]})


if __name__ == '__main__':
    generate()
