#!/usr/bin/env python3
"""Install the pre-extracted, pinned Forge recipe and tag data into a fresh pack."""
import argparse
import hashlib
import json
from pathlib import Path, PurePosixPath
import zipfile

root = Path(__file__).resolve().parents[1]
p = argparse.ArgumentParser(description=__doc__)
p.add_argument('output', type=Path)
a = p.parse_args()
out = a.output.absolute()
if out.exists() or out.is_symlink():
    p.error('output already exists')
manifest = json.loads((root / 'content/reference/forge-47.4.0-data.sha256.json').read_text())
with zipfile.ZipFile(root / 'content/reference/forge-47.4.0-data.zip') as archive:
    names = archive.namelist()
    if len(names) != len(set(names)) or set(names) != set(manifest):
        p.error('bundled Forge data differs from its manifest')
    data = {}
    for name in names:
        path = PurePosixPath(name)
        if not name.startswith('data/') or path.is_absolute() or any(part in ('.', '..') for part in path.parts):
            p.error('invalid bundled path')
        payload = archive.read(name)
        if hashlib.sha256(payload).hexdigest() != manifest[name]:
            p.error('bundled Forge data hash mismatch')
        data[name] = payload
for name, payload in data.items():
    dest = out.joinpath(*PurePosixPath(name).parts)
    dest.parent.mkdir(parents=True, exist_ok=True)
    dest.write_bytes(payload)
print(f'Prepared {len(data)} pinned Forge data files in {out}')
