#!/usr/bin/env python3
"""Prepare an isolated placeholder block solely to test material NBT placement.

The iron-block model and dirt template are test fixtures, not TConstruct behavior.
"""
import argparse
import json
from pathlib import Path
import shutil
import subprocess
import tempfile

p = argparse.ArgumentParser(description=__doc__)
p.add_argument('--game', type=Path, required=True)
p.add_argument('--base', type=Path, required=True)
p.add_argument('--output', type=Path, required=True)
p.add_argument('--reverse-drop-order', action='store_true')
a = p.parse_args()
root = Path(__file__).resolve().parents[3]
game, base, output = a.game.resolve(strict=True), a.base.resolve(strict=True), a.output.absolute()
if output.exists() or output.is_symlink():
    p.error('output must not exist')
with tempfile.TemporaryDirectory(prefix='material-block-content-') as tmp:
    content = Path(tmp) / 'content'
    shutil.copytree(root / 'content', content)
    for src in (Path(__file__).parent / 'overlay').rglob('*'):
        if src.is_file():
            dst = content / src.relative_to(Path(__file__).parent / 'overlay')
            dst.parent.mkdir(parents=True, exist_ok=True)
            # This isolated fixture deliberately replaces the production
            # block assets and loot to exercise the existing NBT/drop probes.
            shutil.copyfile(src, dst)
    if a.reverse_drop_order:
        loot = content / 'data/tconstruct/loot_table/blocks/fake_storage_block.json'
        data = json.loads(loot.read_text())
        data['pools'].reverse()
        loot.write_text(json.dumps(data))
    subprocess.run([str(game), '--tconstruct-prepare', str(base), str(content), str(output)], check=True)
# The fixture uses an dirt template; mark the placeholder as having an
# entity only in this isolated output, so Horizon retains unknown NBT on save.
facts = json.loads((output / 'facts.json').read_text())
path = output / 'gameplay-facts.json'
gameplay = json.loads(path.read_text())
if not gameplay['states'][facts['defaults']['tconstruct:fake_storage_block']]['block_entity']:
    raise ValueError('prepared fixture block is missing its block entity flag')
