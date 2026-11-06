#!/usr/bin/env python3
"""Permit only the official planner's read-only, locked offline metadata call."""
import json, os, sys
from pathlib import Path

ROOT = Path('/workspace/scratch/c7b540ff3393')
REAL = ROOT / '.devtools/rustup/toolchains/1.95.0-x86_64-unknown-linux-gnu/bin/cargo'
MANIFEST = str(ROOT / '.devtools/latest-official-export-20261105/horizon/Cargo.toml')
LOG = Path('/tmp/pluto-main-common-baseline-run-20261106/metadata-argv.jsonl')
expected = ['metadata', '--format-version', '1', '--no-deps', '--manifest-path', MANIFEST, '--offline', '--locked']
args = sys.argv[1:]
# cargo_metadata permits no alternative command or path through this wrapper.
allowed = args == expected
with LOG.open('a') as out:
    out.write(json.dumps({'argv':args,'allowed':allowed,'real_cargo':str(REAL)})+'\n')
if not allowed:
    print('Only exact official locked/offline/no-deps metadata is permitted',file=sys.stderr)
    raise SystemExit(97)
os.execv(str(REAL), [str(REAL), *args])
