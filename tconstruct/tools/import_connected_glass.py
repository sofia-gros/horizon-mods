"""Snapshot the original connected-glass assets for the runtime importer.

Usage: python import_connected_glass.py /path/to/TinkersConstruct
The source repository must contain the revision used by the existing block assets.
"""
import io
import subprocess
import sys
import zipfile
from pathlib import Path

REVISION = 'e14bb5f1794f561ff164f88f84b8b095191c8ecf'
OUTPUT = Path(__file__).resolve().parents[1] / 'content/reference/tconstruct-connected-glass.zip'


def selected(path):
    if '/assets/tconstruct/' not in path:
        return False
    resource = path.split('/assets/tconstruct/', 1)[1]
    if resource.startswith('blockstates/'):
        name = Path(resource).stem
        return name in {'clear_glass', 'clear_glass_pane', 'soul_glass', 'soul_glass_pane'} or name.endswith(('_clear_stained_glass', '_clear_stained_glass_pane')) or name in {
            f'{kind}_{soul}glass{pane}' for kind in ('seared', 'scorched') for soul in ('', 'soul_') for pane in ('', '_pane')
        }
    if resource.startswith('models/item/'):
        name = Path(resource).stem
        return selected('src/generated/resources/assets/tconstruct/blockstates/' + name + '.json')
    if resource.startswith('models/block/'): 
        name = resource[len('models/block/'):]
        return name.startswith(('clear_glass/', 'soul_glass/', 'template/pane/', 'smeltery/glass/', 'smeltery/soul_glass/', 'foundry/glass/', 'foundry/soul_glass/'))
    if resource.startswith('textures/block/'):
        name = resource[len('textures/block/'):]
        return name.startswith(('clear_glass', 'clear_stained_glass', 'soul_glass', 'smeltery/seared_glass', 'smeltery/soul_glass', 'foundry/glass', 'foundry/soul_glass'))
    return False


def snapshot(repo):
    paths = subprocess.check_output(['git', '-C', str(repo), 'ls-tree', '-r', '--name-only', REVISION], text=True).splitlines()
    paths = sorted(p for p in paths if selected(p))
    if not paths:
        raise ValueError('connected glass assets are absent from the source revision')
    result = io.BytesIO()
    with zipfile.ZipFile(result, 'w', compression=zipfile.ZIP_DEFLATED) as out:
        for path in paths:
            blob = subprocess.check_output(['git', '-C', str(repo), 'show', f'{REVISION}:{path}'])
            info = zipfile.ZipInfo(path, (1980, 1, 1, 0, 0, 0))
            info.compress_type = zipfile.ZIP_DEFLATED
            out.writestr(info, blob)
    return result.getvalue()


if __name__ == '__main__':
    if len(sys.argv) != 2:
        raise SystemExit(__doc__)
    OUTPUT.write_bytes(snapshot(Path(sys.argv[1])))
    print(f'Wrote {OUTPUT}')
