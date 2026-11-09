#!/usr/bin/env python3
"""Fetches the original Tinkers' Construct source (MIT licence) that the port is measured against, into `.local/tconstruct-source`.

    python tools/fetch_original.py

The port's specification is this Java source and its generated data files (`src/generated/resources`); it is not part of this repository.
The commit is pinned: it is the one the inventories (`docs/inventory/*.json`) and the data tests were made from. Safe to run again.
"""
import subprocess
import sys
from pathlib import Path

URL = 'https://github.com/SlimeKnights/TinkersConstruct.git'
COMMIT = 'f4e77d43f39ba1047bf93ae816b7e1beaab514c2'
TARGET = Path(__file__).resolve().parents[1] / '.local' / 'tconstruct-source'


def git(*args, cwd=TARGET):
    result = subprocess.run(['git', *args], cwd=cwd, capture_output=True, text=True, encoding='utf-8', errors='replace')
    if result.returncode != 0:
        sys.exit(f'FAILED: git {" ".join(args)}\n{result.stdout}{result.stderr}')
    return result.stdout.strip()


def main():
    if (TARGET / 'src/main/java').is_dir() and (TARGET / '.git').exists() and git('rev-parse', 'HEAD') == COMMIT:
        print(f'the original source is in place at {COMMIT[:10]}: {TARGET}')
        return 0
    TARGET.mkdir(parents=True, exist_ok=True)
    if not (TARGET / '.git').exists():
        git('init', '-q')
        git('remote', 'add', 'origin', URL)
    git('fetch', '-q', '--depth', '1', 'origin', COMMIT)
    git('checkout', '-q', '--force', 'FETCH_HEAD')
    head = git('rev-parse', 'HEAD')
    if head != COMMIT:
        sys.exit(f'the checkout is {head}, expected {COMMIT}')
    print(f'fetched the original source at {COMMIT[:10]} into {TARGET}')
    return 0


if __name__ == '__main__':
    sys.exit(main())
