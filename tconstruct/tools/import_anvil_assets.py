"""Snapshot the pinned source models, blockstates and textures for both anvils."""
import io
import subprocess
import sys
import zipfile
from pathlib import Path

REVISION = 'f4e77d43f39ba1047bf93ae816b7e1beaab514c2'
OUTPUT = Path(__file__).resolve().parents[1] / 'content/reference/tconstruct-anvil-assets.zip'
FILES = [
    *(f'blockstates/{name}.json' for name in ('tinkers_anvil', 'scorched_anvil')),
    *(f'models/item/{name}.json' for name in ('tinkers_anvil', 'scorched_anvil')),
    *(f'models/block/table/{name}.json' for name in ('tinkers_anvil', 'scorched_anvil')),
    *(f'textures/block/table/{name}.png' for name in ('anvil_base', 'anvil_base_upper', 'scorched_anvil_base', 'scorched_anvil_base_upper')),
    'textures/block/storage/fallback.png',
]


def snapshot(repo):
    output = io.BytesIO()
    with zipfile.ZipFile(output, 'w', zipfile.ZIP_DEFLATED) as archive:
        for suffix in sorted(FILES):
            path = 'assets/tconstruct/' + suffix
            data = subprocess.check_output(['git', '-C', str(repo), 'show', f'{REVISION}:src/main/resources/{path}'])
            info = zipfile.ZipInfo(path, (1980, 1, 1, 0, 0, 0))
            info.compress_type = zipfile.ZIP_DEFLATED
            archive.writestr(info, data)
    return output.getvalue()


if __name__ == '__main__':
    if len(sys.argv) != 2:
        raise SystemExit('Usage: python import_anvil_assets.py /path/to/TinkersConstruct')
    OUTPUT.write_bytes(snapshot(Path(sys.argv[1])))
    print(f'Wrote {OUTPUT}')
