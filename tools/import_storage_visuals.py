#!/usr/bin/env python3
"""Export the pinned original material storage block's visual resources."""
import argparse
import hashlib
from pathlib import Path
import subprocess

REV = 'f4e77d43f39ba1047bf93ae816b7e1beaab514c2'
ROOTS = ('src/main/resources/', 'src/generated/resources/')
EXACT = {
    'assets/tconstruct/blockstates/fake_storage_block.json',
    'assets/tconstruct/models/block/storage/fake.json',
    'assets/tconstruct/models/item/fake_storage_block.json',
    'assets/tconstruct/textures/block/storage/fallback.png',
}
PREFIX = 'assets/tconstruct/textures/block/storage/fallback_'

p = argparse.ArgumentParser()
p.add_argument('repo', type=Path)
p.add_argument('output', type=Path)
p.add_argument('manifest', type=Path)
a = p.parse_args()
if a.output.exists() or a.output.is_symlink() or a.manifest.exists() or a.manifest.is_symlink():
    raise ValueError('output and manifest must not exist')
repo = a.repo.resolve(strict=True)
def git(*args):
    return subprocess.check_output(['git', '-C', str(repo), *args])
names = git('ls-tree', '-r', '--name-only', REV).decode().splitlines()
selected = {}
for name in names:
    root = next((root for root in ROOTS if name.startswith(root)), None)
    if root is None:
        continue
    relative = name[len(root):]
    if relative in EXACT or (relative.startswith(PREFIX) and relative.endswith('.png')):
        if relative in selected:
            raise ValueError(f'duplicate source path: {relative}')
        selected[relative] = name
if not EXACT.issubset(selected) or len(selected) != 10:
    raise ValueError('unexpected pinned storage visual resources')
a.output.mkdir(parents=True)
rows = []
for relative, source in sorted(selected.items()):
    data = git('show', f'{REV}:{source}')
    target = a.output / relative
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_bytes(data)
    rows.append(f'{hashlib.sha256(data).hexdigest()}  {relative}\n')
a.manifest.write_text(''.join(rows))
