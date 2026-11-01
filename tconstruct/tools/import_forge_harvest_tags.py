"""Snapshot the Forge 47.4.0 block-tag closure used by source harvest predicates.

Usage: python3 import_forge_harvest_tags.py /path/to/forge-47.4.0-universal.jar
"""
import hashlib
import io
import json
from pathlib import Path
import sys
import zipfile

ROOT = Path(__file__).resolve().parents[1]
SOURCE_SHA256 = '2b0e97b22fd4d043dbbcc8e0a44ef0d58915689ed9224e4cd28d5c4976a16bcc'
OUTPUT = ROOT / 'content/reference/forge-47.4.0-harvest-tags.zip'

def snapshot(jar):
    data = Path(jar).read_bytes()
    if hashlib.sha256(data).hexdigest() != SOURCE_SHA256:
        raise ValueError('unexpected Forge 47.4.0 universal JAR hash')
    original = zipfile.ZipFile(ROOT / 'content/reference/tconstruct-block-tags.zip')
    source = zipfile.ZipFile(io.BytesIO(data))
    pending = ['forge:glass', 'forge:glass_panes', 'forge:mineable/sword', 'forge:mineable/shears']
    visited, entries = set(), {}
    while pending:
        tag = pending.pop()
        if tag in visited or not tag.startswith('forge:'): continue
        visited.add(tag)
        name = 'data/forge/tags/blocks/' + tag.split(':', 1)[1] + '.json'
        for archive in (source, original):
            if name not in archive.namelist(): continue
            payload = archive.read(name)
            if archive is source: entries[name] = payload
            raw = json.loads(payload)
            for value in raw.get('values', []) + raw.get('remove', []):
                item = value if isinstance(value, str) else value.get('id', '')
                if item.startswith('#forge:'): pending.append(item[1:])
    if not entries: raise ValueError('missing Forge harvest tag closure')
    out = io.BytesIO()
    with zipfile.ZipFile(out, 'w', zipfile.ZIP_DEFLATED) as archive:
        for name, payload in sorted(entries.items()):
            info = zipfile.ZipInfo(name, (1980, 1, 1, 0, 0, 0))
            info.compress_type = zipfile.ZIP_DEFLATED
            archive.writestr(info, payload)
    return out.getvalue()

if __name__ == '__main__':
    if len(sys.argv) != 2: raise SystemExit(__doc__)
    OUTPUT.write_bytes(snapshot(sys.argv[1]))
    print(f'Wrote {OUTPUT}')
