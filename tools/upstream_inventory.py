#!/usr/bin/env python3
"""Create a reproducible index of every tracked file in the pinned upstream tree."""
import argparse
import gzip
import json
from pathlib import Path
import subprocess

root = Path(__file__).resolve().parents[1]
revision = json.loads((root / 'compatibility.json').read_text())['tconstruct_revision']
p = argparse.ArgumentParser()
p.add_argument('repository', type=Path)
a = p.parse_args()
paths = subprocess.check_output(['git', '-C', str(a.repository), 'ls-tree', '-r', '--name-only', revision])
(root / 'docs/upstream-tree.txt.gz').write_bytes(gzip.compress(paths, compresslevel=9, mtime=0))
print(f'{len(paths.splitlines())} tracked paths indexed at {revision}')
