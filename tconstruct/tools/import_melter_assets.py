"""Snapshot original melter/casting model dependencies and GUI assets.

Usage: python import_melter_assets.py /path/to/TinkersConstruct
The source revision matches the existing block asset snapshot.
"""
import io
import json
import re
import subprocess
import sys
import zipfile
from pathlib import Path

REVISION = 'e14bb5f1794f561ff164f88f84b8b095191c8ecf'
OUTPUT = Path(__file__).resolve().parents[1] / 'content/reference/tconstruct-melter-assets.zip'
BLOCKS = ('seared_melter', 'seared_heater', 'seared_faucet', 'seared_table', 'seared_basin')


def snapshot(repo):
    paths = subprocess.check_output(['git', '-C', str(repo), 'ls-tree', '-r', '--name-only', REVISION], text=True).splitlines()
    available = {p.split('/assets/tconstruct/', 1)[1]: p for p in paths if '/assets/tconstruct/' in p}
    found = {}

    def add(name):
        if name in found:
            return
        source = available.get(name)
        if source is None:
            raise ValueError(f'missing source asset: {name}')
        data = subprocess.check_output(['git', '-C', str(repo), 'show', f'{REVISION}:{source}'])
        found[name] = data
        if not name.endswith('.json'):
            return
        value = json.loads(data)
        def visit(node):
            if isinstance(node, dict):
                for key, value in node.items():
                    if isinstance(value, str) and value.startswith('tconstruct:'):
                        ident = value.removeprefix('tconstruct:')
                        if key in ('parent', 'model'):
                            add('models/' + ident + '.json')
                        elif name.startswith('models/') and key not in ('loader', 'render_type') and ident.startswith(('block/', 'item/')):
                            add('textures/' + ident + '.png')
                    else:
                        visit(value)
            elif isinstance(node, list):
                for child in node:
                    visit(child)
        visit(value)

    for name in BLOCKS:
        add('blockstates/' + name + '.json')
        add('models/item/' + name + '.json')
    for name in ('textures/gui/melter.png', 'textures/gui/icons.png', 'textures/fluid/molten/still.png', 'textures/fluid/molten/still.png.mcmeta'):
        add(name)
    # Source FluidTextureProvider selects textures by category. Preserve the
    # unanimated first frame as supplied by the source; do not invent colors.
    def java(path):
        return subprocess.check_output(['git', '-C', str(repo), 'show', f'{REVISION}:{path}'], text=True)
    fluids = java('src/main/java/slimeknights/tconstruct/fluids/TinkerFluids.java')
    provider = java('src/main/java/slimeknights/tconstruct/fluids/data/FluidTextureProvider.java')
    ids = dict(re.findall(r'\b(\w+)\s*=\s*FLUIDS\.register(?:Metal)?\("([^"\n]+)"', fluids))
    categories = {'ore': 'ore', 'alloy': 'alloy', 'compatOre': 'compat_ore', 'compatAlloy': 'compat_alloy', 'stone': 'stone'}
    texture_map = {}
    for category, variable in re.findall(r'\b(ore|alloy|compatOre|compatAlloy|stone)\(TinkerFluids\.(\w+)\)', provider):
        fluid_id = ids.get(variable)
        if not fluid_id or not fluid_id.startswith('molten_'):
            continue
        texture = f'textures/fluid/molten/{categories[category]}/{fluid_id.removeprefix("molten_")}/still.png'
        if texture in available:
            add(texture)
            texture_map['tconstruct:' + fluid_id] = texture
    found['melter-fluid-textures.json'] = (json.dumps(texture_map, sort_keys=True, indent=2) + '\n').encode()
    result = io.BytesIO()
    with zipfile.ZipFile(result, 'w', compression=zipfile.ZIP_DEFLATED) as out:
        for path, data in sorted(found.items()):
            info = zipfile.ZipInfo('assets/tconstruct/' + path, (1980, 1, 1, 0, 0, 0))
            info.compress_type = zipfile.ZIP_DEFLATED
            out.writestr(info, data)
    return result.getvalue()


if __name__ == '__main__':
    if len(sys.argv) != 2:
        raise SystemExit(__doc__)
    OUTPUT.write_bytes(snapshot(Path(sys.argv[1])))
    print(f'Wrote {OUTPUT}')
