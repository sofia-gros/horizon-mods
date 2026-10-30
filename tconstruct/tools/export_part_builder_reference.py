#!/usr/bin/env python3
"""Export pinned Part Builder and material input JSON from the reference catalog.

The files are source data, not evidence that the game loads those recipes yet.
Existing differing files are never overwritten.
"""
import gzip
import json
from pathlib import Path

root = Path(__file__).resolve().parents[1]
compat = json.loads((root / 'compatibility.json').read_text())
with gzip.open(root / 'docs/upstream-recipes.json.gz', 'rt', encoding='utf-8') as f:
    catalog = json.load(f)
if catalog['revision'] != compat['tconstruct_revision']:
    raise SystemExit('reference catalog revision does not match compatibility.json')
prefix = 'src/generated/resources/'
entries = catalog['groups']['part_builder'] + [e for e in catalog['groups']['materials'] if e['data'].get('type') == 'tconstruct:material']
for entry in entries:
    path = entry['source']
    if not path.startswith(prefix + 'data/tconstruct/') or '..' in Path(path).parts:
        raise SystemExit(f'unsafe reference path: {path}')
    output = root / 'content' / path.removeprefix(prefix)
    encoded = (json.dumps(entry['data'], ensure_ascii=False, indent=2) + '\n').encode()
    if output.exists() and output.read_bytes() != encoded:
        raise SystemExit(f'refusing to overwrite existing differing file: {output}')
    output.parent.mkdir(parents=True, exist_ok=True)
    if not output.exists():
        output.write_bytes(encoded)
print(f'exported {len(entries)} source recipe files at {catalog["revision"]}')
