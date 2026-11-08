#!/usr/bin/env python3
"""Lists the blocks and items of the original (by its language file) that the prepared game does not know.

Usage: gap_report.py [--resources DIR]   (default .local/smoke-build/game-resources)
"""
import argparse
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
p = argparse.ArgumentParser()
p.add_argument('--resources', type=Path, default=ROOT / '.local/smoke-build/game-resources')
p.add_argument('--lang', type=Path, default=ROOT / '.local/tconstruct-source/src/main/resources/assets/tconstruct/lang/en_us.json')
a = p.parse_args()
lang = json.loads(a.lang.read_text(encoding='utf-8'))
known = set()
for name in ('items.json', 'registries.json'):
    known |= set(re.findall(r'"tconstruct:([a-z0-9_/]+)"', (a.resources / name).read_text(encoding='utf-8')))
wanted = {k.split('.', 2)[2] for k in lang if k.startswith(('item.tconstruct.', 'block.tconstruct.')) and k.count('.') == 2}
missing = sorted(wanted - known)
print(f'{len(wanted)} in the original, {len(missing)} not known to the game')
print(' '.join(missing))
sys.exit(0)
