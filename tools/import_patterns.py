#!/usr/bin/env python3
"""Export the pinned upstream pattern sprites without overwriting outputs."""
import argparse
import hashlib
from pathlib import Path
import subprocess

REV = 'f4e77d43f39ba1047bf93ae816b7e1beaab514c2'
PREFIX = 'src/main/resources/assets/tconstruct/textures/gui/tinker_pattern/'

p = argparse.ArgumentParser()
p.add_argument('repo', type=Path)
p.add_argument('output', type=Path)
p.add_argument('manifest', type=Path)
a = p.parse_args()
if a.output.exists() or a.output.is_symlink() or a.manifest.exists() or a.manifest.is_symlink():
    raise ValueError('output and manifest must not exist')
def git(*args):
    return subprocess.check_output(['git', '-C', str(a.repo.resolve(strict=True)), *args])
paths = git('ls-tree', '-r', '--name-only', REV, '--', PREFIX).decode().splitlines()
if not paths or any(not path.startswith(PREFIX) or '/' in path[len(PREFIX):] or not path.endswith('.png') for path in paths):
    raise ValueError('unexpected pattern files')
a.output.mkdir(parents=True)
rows = []
for path in paths:
    name = path[len(PREFIX):]
    contents = git('show', f'{REV}:{path}')
    (a.output / name).write_bytes(contents)
    rows.append(f'{hashlib.sha256(contents).hexdigest()}  {name}\n')
a.manifest.write_text(''.join(rows))
