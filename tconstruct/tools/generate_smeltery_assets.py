"""Stage source Smeltery block shells for later machine behavior.

The ordinary models contain the source's static elements and textures. Dynamic
liquid, formed-state and connected-texture rendering are separate runtime work.
"""
import json
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CONTENT = ROOT / 'content'
ARCHIVE = CONTENT / 'reference/tconstruct-smeltery-assets.zip'


def emit(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, separators=(',', ':')) + '\n')


def generate(root=CONTENT):
    with zipfile.ZipFile(ARCHIVE) as archive:
        source = {p.removeprefix('assets/tconstruct/'): archive.read(p) for p in archive.namelist()}
    assets = root / 'assets/tconstruct'
    for name, payload in source.items():
        path = assets / name
        if name.startswith('textures/'):
            path.parent.mkdir(parents=True, exist_ok=True)
            if path.exists() and path.read_bytes() != payload:
                raise ValueError(f'conflicting source texture {name}')
            path.write_bytes(payload)
        elif name.startswith('models/') and name.endswith('.json'):
            model = json.loads(payload)
            loader = model.pop('loader', None)
            if loader not in (None, 'mantle:retextured', 'mantle:connected', 'tconstruct:tank', 'tconstruct:fluid_texture'):
                raise ValueError(f'unsupported source loader {loader} in {name}')
            for key in ('colors', 'connection', 'retextured', 'fluids', 'fluid'):
                model.pop(key, None)
            if path.exists() and json.loads(path.read_bytes()) != model:
                raise ValueError(f'conflicting source model {name}')
            emit(path, model)
    for name, payload in source.items():
        if not name.startswith('blockstates/'):
            continue
        block = Path(name).stem
        state = json.loads(payload)
        if block in ('smeltery_controller', 'foundry_controller', 'seared_drain', 'seared_chute', 'seared_duct', 'seared_ladder', 'scorched_drain', 'scorched_chute', 'scorched_duct', 'scorched_ladder'):
            variants = {}
            for raw, model in state['variants'].items():
                values = dict(pair.split('=', 1) for pair in raw.split(','))
                if block in ('smeltery_controller', 'foundry_controller'):
                    # The template currently carries the active flag only.
                    if values.get('in_structure') != 'true':
                        continue
                    key = f'facing={values["facing"]},lit={values["active"]}'
                elif block in ('seared_ladder', 'scorched_ladder'):
                    if values.get('bottom') != 'false':
                        continue
                    key = f'facing={values["facing"]}'
                else:
                    if values.get('in_structure') != 'false':
                        continue
                    key = f'facing={values["facing"]}'
                variants[key] = model
            if len(variants) != (8 if block in ('smeltery_controller', 'foundry_controller') else 4):
                raise ValueError(f'unexpected source state variants for {block}')
            state = {'variants': variants}
        emit(assets / name, state)
        emit(assets / f'items/{block}.json', {'model': {'type': 'minecraft:model', 'model': f'tconstruct:item/{block}'}})
        template = ('minecraft:furnace' if block in ('smeltery_controller', 'foundry_controller') else
                    'minecraft:ladder' if block in ('seared_ladder', 'scorched_ladder') else
                    'minecraft:glass' if block.endswith(('_tinted_glass', '_fuel_tank', '_fuel_gauge', '_ingot_tank', '_ingot_gauge')) else
                    'minecraft:white_glazed_terracotta')
        emit(root / f'data/tconstruct/horizon/block/{block}.json', {'template': template, 'properties': {
            'hardness': 3.0, 'explosion_resistance': 9.0, 'requires_tool': True, 'sounds': 'minecraft:stone'}})
        emit(root / f'data/tconstruct/loot_table/blocks/{block}.json', {
            'type': 'minecraft:block', 'pools': [{'condition': {'type': 'minecraft:survives_explosion'},
            'entries': [{'type': 'minecraft:item', 'name': f'tconstruct:{block}'}], 'rolls': 1}]})


if __name__ == '__main__':
    generate()
