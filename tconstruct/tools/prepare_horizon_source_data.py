#!/usr/bin/env python3
"""Adapt prepared Horizon 26.3 Minecraft data to the source-pack layout."""
import argparse
from pathlib import Path
import shutil

p = argparse.ArgumentParser(description=__doc__)
p.add_argument('resources', type=Path)
p.add_argument('output', type=Path)
a = p.parse_args()
root = a.resources.resolve(strict=True)
out = a.output.absolute()
if out.exists() or out.is_symlink() or out.is_relative_to(root) or root.is_relative_to(out):
    p.error('output must not exist and must be separate from resources')
if not (root / 'horizon-resources.json').is_file():
    p.error('expected prepared Horizon 26.3 resources')
mappings = [('recipe', 'recipes'), ('tags/item', 'tags/items'), ('tags/fluid', 'tags/fluids'), ('tags/block', 'tags/blocks')]
files = []
for source, destination in mappings:
    base = root / 'data/minecraft' / source
    if not base.is_dir():
        p.error(f'missing prepared data directory: {base}')
    found = sorted(base.rglob('*.json'))
    if not found:
        p.error(f'empty prepared data directory: {base}')
    for path in found:
        files.append((path, out / 'data/minecraft' / destination / path.relative_to(base)))
for path, dest in files:
    dest.parent.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(path, dest)
print(f'Prepared {len(files)} Horizon 26.3 Minecraft recipe and tag files in {out}')
