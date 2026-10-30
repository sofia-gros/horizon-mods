#!/usr/bin/env python3
"""Export pinned TConstruct tool models and generated material sprites."""
import argparse
import hashlib
import io
import json
from pathlib import Path, PurePosixPath
import subprocess
import tarfile
import zipfile

REV = 'f4e77d43f39ba1047bf93ae816b7e1beaab514c2'
PREFIXES = ('src/main/resources/assets/tconstruct/models/item/',
            'src/generated/resources/assets/tconstruct/models/item/tool/',
            'src/generated/resources/assets/tconstruct/textures/item/tool/')
p = argparse.ArgumentParser(description=__doc__)
p.add_argument('repo', type=Path)
p.add_argument('output', type=Path)
p.add_argument('manifest', type=Path)
a = p.parse_args()
if a.output.exists() or a.manifest.exists():
    raise ValueError('output and manifest must not exist')
raw = subprocess.check_output(['git', '-C', str(a.repo.resolve(strict=True)), 'archive', REV, *PREFIXES])
rows = []
with zipfile.ZipFile(Path(__file__).resolve().parents[1] / 'content/reference/tconstruct-recipes.zip') as recipes:
    names = set()
    for path in recipes.namelist():
        if path.endswith('.json'):
            value = json.loads(recipes.read(path))
            if value.get('type') == 'tconstruct:tool_building':
                names.add(value['result'].split(':', 1)[1] + '.json')
with tarfile.open(fileobj=io.BytesIO(raw)) as source, zipfile.ZipFile(a.output, 'x', compression=zipfile.ZIP_DEFLATED, compresslevel=9) as target:
    files = sorted((entry for entry in source if entry.isfile()), key=lambda entry: entry.name)
    for entry in files:
        prefix = next((p for p in PREFIXES if entry.name.startswith(p)), None)
        if prefix is None or '..' in PurePosixPath(entry.name).parts:
            raise ValueError('unexpected source path')
        name = entry.name.split('/resources/', 1)[1]
        if prefix == PREFIXES[0] and not name.startswith('assets/tconstruct/models/item/base/') and PurePosixPath(name).name not in names:
            continue
        value = source.extractfile(entry).read()
        info = zipfile.ZipInfo(name, (1980, 1, 1, 0, 0, 0))
        info.compress_type = zipfile.ZIP_DEFLATED
        target.writestr(info, value, compresslevel=9)
        rows.append(f'{hashlib.sha256(value).hexdigest()}  {name}\n')
a.manifest.write_text(''.join(rows))
