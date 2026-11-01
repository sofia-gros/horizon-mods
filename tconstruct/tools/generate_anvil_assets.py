"""Generate the pinned source anvil shells; material retexturing is separate."""
import json
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
ARCHIVE = ROOT / 'content/reference/tconstruct-anvil-assets.zip'


def generate(root):
    assets = root / 'assets/tconstruct'
    with zipfile.ZipFile(ARCHIVE) as archive:
        for name in sorted(archive.namelist()):
            relative = name.removeprefix('assets/tconstruct/')
            dest = assets / relative
            dest.parent.mkdir(parents=True, exist_ok=True)
            data = archive.read(name)
            if relative.startswith('models/block/table/'):
                model = json.loads(data)
                if model.pop('loader', None) != 'tconstruct:material_block':
                    raise ValueError(f'unexpected source anvil loader: {name}')
                model.pop('retextured', None)
                data = (json.dumps(model, separators=(',', ':')) + '\n').encode()
            dest.write_bytes(data)
    for block in ('tinkers_anvil', 'scorched_anvil'):
        outputs = {
            f'assets/tconstruct/items/{block}.json': {'model': {'type': 'minecraft:model', 'model': f'tconstruct:item/{block}'}},
            f'data/tconstruct/horizon/block/{block}.json': {'template': 'minecraft:white_glazed_terracotta', 'properties': {
                'hardness': 5.0, 'explosion_resistance': 1200.0, 'requires_tool': True, 'sounds': 'minecraft:anvil'}},
            f'data/tconstruct/loot_table/blocks/{block}.json': {'type': 'minecraft:block', 'pools': [
                {'condition': {'type': 'minecraft:survives_explosion'}, 'entries': [{'type': 'minecraft:item', 'name': f'tconstruct:{block}'}], 'rolls': 1}]},
        }
        for name, value in outputs.items():
            dest = root / name
            dest.parent.mkdir(parents=True, exist_ok=True)
            dest.write_text(json.dumps(value, separators=(',', ':')) + '\n')


if __name__ == '__main__':
    generate(ROOT / 'content')
