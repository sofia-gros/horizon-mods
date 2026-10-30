#!/usr/bin/env python3
"""Export pinned TConstruct generated station-layout data, byte for byte."""
import argparse
import hashlib
from pathlib import Path
import subprocess
import zipfile

REV = 'f4e77d43f39ba1047bf93ae816b7e1beaab514c2'
PREFIX = 'src/generated/resources/data/tconstruct/tinkering/station_layouts/'
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
if not paths or any(not path.startswith(PREFIX) or not path.endswith('.json') or '..' in Path(path).parts for path in paths):
    raise ValueError('unexpected layout files')
rows = []
with zipfile.ZipFile(a.output, 'x', compression=zipfile.ZIP_DEFLATED, compresslevel=9) as archive:
    for path in paths:
        name = path.removeprefix('src/generated/resources/')
        value = git('show', f'{REV}:{path}')
        info = zipfile.ZipInfo(name, (1980, 1, 1, 0, 0, 0))
        info.compress_type = zipfile.ZIP_DEFLATED
        archive.writestr(info, value, compresslevel=9)
        rows.append(f'{hashlib.sha256(value).hexdigest()}  {name}\n')
a.manifest.write_text(''.join(rows))
