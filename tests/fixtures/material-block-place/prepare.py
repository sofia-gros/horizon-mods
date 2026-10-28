#!/usr/bin/env python3
"""Prepare an isolated placeholder block solely to test material NBT placement.

The iron-block model and template are test fixtures, not TConstruct behavior.
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
            if dst.exists():
                raise ValueError(f'fixture would overwrite a real content file: {dst}')
            shutil.copyfile(src, dst)
    subprocess.run([str(game), '--tconstruct-prepare', str(base), str(content), str(output)], check=True)
# The fixture uses an iron block template; mark the placeholder as having an
# entity only in this isolated output, so Horizon retains unknown NBT on save.
facts = json.loads((output / 'facts.json').read_text())
path = output / 'gameplay-facts.json'
gameplay = json.loads(path.read_text())
gameplay['states'][facts['defaults']['tconstruct:fake_storage_block']]['block_entity'] = True
path.write_text(json.dumps(gameplay))
