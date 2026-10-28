#!/usr/bin/env python3
"""Catalog pinned upstream material/part recipes and station layouts verbatim."""
import argparse
import gzip
import io
import json
from pathlib import Path
import subprocess
import tarfile

root = Path(__file__).resolve().parents[1]
revision = json.loads((root / 'compatibility.json').read_text())['tconstruct_revision']
p = argparse.ArgumentParser()
p.add_argument('repository', type=Path)
a = p.parse_args()
prefix = 'src/generated/resources/data/tconstruct/'
groups = {'materials': prefix + 'recipes/tools/materials/',
          'part_builder': prefix + 'recipes/tools/parts/builder/',
          'station_layouts': prefix + 'tinkering/station_layouts/'}
archive = subprocess.check_output(['git', '-C', str(a.repository), 'archive', '--format=tar', revision, *groups.values()])
entries = {key: [] for key in groups}
with tarfile.open(fileobj=io.BytesIO(archive), mode='r:') as tar:
    for member in tar:
        if not member.isfile() or not member.name.endswith('.json'):
            continue
        for key, path in groups.items():
            if member.name.startswith(path):
                entries[key].append({'source': member.name, 'status': 'unreviewed', 'data': json.load(tar.extractfile(member))})
                break
for group in entries.values():
    group.sort(key=lambda entry: entry['source'])
payload = json.dumps({'revision': revision, 'groups': entries}, ensure_ascii=False, sort_keys=True, separators=(',', ':')).encode()
(root / 'docs/upstream-recipes.json.gz').write_bytes(gzip.compress(payload, compresslevel=9, mtime=0))
print(', '.join(f'{key}: {len(value)}' for key, value in entries.items()))
