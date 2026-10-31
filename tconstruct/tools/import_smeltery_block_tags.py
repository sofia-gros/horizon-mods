"""Snapshot source block tags required by the Smeltery registry.

Usage: python import_smeltery_block_tags.py /path/to/TinkersConstruct
"""
import io
import subprocess
import sys
import tarfile
import zipfile
from pathlib import Path

REVISION = 'e14bb5f1794f561ff164f88f84b8b095191c8ecf'
OUTPUT = Path(__file__).resolve().parents[1] / 'content/reference/tconstruct-block-tags.zip'
PREFIXES = ('src/main/resources/data/', 'src/generated/resources/data/')


def snapshot(repo):
    tree = subprocess.check_output(['git', '-C', str(repo), 'ls-tree', '-r', '--name-only', REVISION], text=True).splitlines()
    paths = [p for p in tree if p.startswith(PREFIXES) and '/tags/blocks/' in p and p.endswith('.json')]
    if not paths:
        raise ValueError('no source block tags found')
    archive = subprocess.check_output(['git', '-C', str(repo), 'archive', REVISION, *paths])
    entries = {}
    with tarfile.open(fileobj=io.BytesIO(archive)) as tar:
        for entry in tar:
            if not entry.isfile():
                continue
            path = next(entry.name.removeprefix(prefix) for prefix in PREFIXES if entry.name.startswith(prefix))
            name = 'data/' + path
            data = tar.extractfile(entry).read()
            if name in entries and entries[name] != data:
                raise ValueError(f'conflicting source block tag: {name}')
            entries[name] = data
    out = io.BytesIO()
    with zipfile.ZipFile(out, 'w', zipfile.ZIP_DEFLATED) as z:
        for name, data in sorted(entries.items()):
            info = zipfile.ZipInfo(name, (1980, 1, 1, 0, 0, 0))
            info.compress_type = zipfile.ZIP_DEFLATED
            z.writestr(info, data)
    return out.getvalue()


if __name__ == '__main__':
    if len(sys.argv) != 2:
        raise SystemExit(__doc__)
    OUTPUT.write_bytes(snapshot(Path(sys.argv[1])))
    print(f'Wrote {OUTPUT}')
