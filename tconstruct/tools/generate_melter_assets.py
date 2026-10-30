"""Stage source melter assets for Horizon's existing block-state templates.

The source tank loader is retained in the reference archive. Its static shell
is emitted as ordinary geometry; the liquid surface requires the runtime
renderer and is not supplied by this converter.
"""
import json
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CONTENT = ROOT / 'content'
ARCHIVE = CONTENT / 'reference/tconstruct-melter-assets.zip'
BLOCKS = ('seared_melter', 'seared_heater', 'seared_faucet', 'seared_table', 'seared_basin')


def emit(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, separators=(',', ':')) + '\n')


def generate(root=CONTENT):
    with zipfile.ZipFile(ARCHIVE) as archive:
        source = {p.removeprefix('assets/tconstruct/'): archive.read(p) for p in archive.namelist()}
    assets = root / 'assets/tconstruct'
    for name, payload in source.items():
        if name.startswith('textures/'):
            path = assets / name
            path.parent.mkdir(parents=True, exist_ok=True)
            if path.exists() and path.read_bytes() != payload:
                raise ValueError(f'source asset conflicts with existing content: {name}')
            path.write_bytes(payload)
        elif name.startswith('models/') and name.endswith('.json'):
            model = json.loads(payload)
            loader = model.pop('loader', None)
            if loader and not (name == 'models/block/template/half_tank.json' and loader == 'tconstruct:tank'):
                raise ValueError(f'unsupported model loader {loader} in {name}')
            emit(assets / name, model)
    for block in BLOCKS:
        state = json.loads(source[f'blockstates/{block}.json'])
        if block in ('seared_melter', 'seared_heater'):
            variants = {}
            for raw, model in state['variants'].items():
                values = dict(pair.split('=', 1) for pair in raw.split(','))
                if block == 'seared_melter' and values.get('in_structure') != 'true':
                    continue
                variants[f'facing={values["facing"]},lit={values["active"]}'] = model
            if len(variants) != 8:
                raise ValueError(f'unexpected source states for {block}')
            state = {'variants': variants}
        emit(assets / f'blockstates/{block}.json', state)
        emit(assets / f'items/{block}.json', {'model': {'type': 'minecraft:model', 'model': f'tconstruct:item/{block}'}})
        template = 'minecraft:furnace' if block in ('seared_melter', 'seared_heater') else 'minecraft:hopper' if block == 'seared_faucet' else 'minecraft:white_glazed_terracotta'
        emit(root / f'data/tconstruct/horizon/block/{block}.json', {'template': template, 'properties': {
            'hardness': 8.0 if block in ('seared_melter', 'seared_heater') else 3.0,
            'explosion_resistance': 28.0 if block in ('seared_melter', 'seared_heater') else 9.0,
            'requires_tool': True, 'sounds': 'minecraft:iron_block'}})
        emit(root / f'data/tconstruct/loot_table/blocks/{block}.json', {
            'type': 'minecraft:block', 'pools': [{'condition': {'type': 'minecraft:survives_explosion'},
            'entries': [{'type': 'minecraft:item', 'name': f'tconstruct:{block}'}], 'rolls': 1}]})


if __name__ == '__main__':
    generate()
