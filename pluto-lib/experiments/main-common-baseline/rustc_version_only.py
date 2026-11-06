#!/usr/bin/env python3
"""Draft only: permit version observation, never compilation."""
import json, os, sys
from pathlib import Path
REAL = Path('/workspace/scratch/c7b540ff3393/.devtools/rustup/toolchains/1.95.0-x86_64-unknown-linux-gnu/bin/rustc')
LOG = Path('/tmp/pluto-main-common-baseline-run-20261106/rustc-argv.jsonl')
args = sys.argv[1:]
allowed = args in [['--version'], ['-vV']]
with LOG.open('a') as out:
    out.write(json.dumps({'argv':args,'allowed':allowed,'real_rustc':str(REAL)})+'\n')
if not allowed:
    print('Only rustc version queries are permitted; compilation is forbidden',file=sys.stderr)
    raise SystemExit(97)
os.execv(str(REAL), [str(REAL), *args])
