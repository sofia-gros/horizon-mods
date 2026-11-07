#!/usr/bin/env python3
"""Fail when line endings can go wrong again.

Checks (see .gitattributes): a tracked text file must be LF in the index (except files marked eol=crlf), and .gitattributes
must force LF in working trees so Windows' core.autocrlf cannot turn them into CRLF. Run from the repository root.

  check_line_endings.py            # exit 1 with the offending files
  check_line_endings.py --fix      # tell how to repair a working tree that was checked out with CRLF
"""
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
REQUIRED_RULE = '* text=auto eol=lf'


def parse(eol_listing: str):
    """Yield (path, index_eol, worktree_eol, attr) from `git ls-files --eol` output."""
    for line in eol_listing.splitlines():
        meta, _, path = line.partition('\t')
        fields = meta.split()
        if len(fields) < 3 or not fields[0].startswith('i/'):
            continue
        yield path, fields[0][2:], fields[1][2:], ' '.join(fields[2:])


def violations(eol_listing: str, attributes: str):
    found = []
    if REQUIRED_RULE not in attributes.splitlines():
        found.append(f'.gitattributes must contain the rule "{REQUIRED_RULE}"')
    for path, index_eol, worktree_eol, attr in parse(eol_listing):
        crlf_allowed = 'eol=crlf' in attr
        if index_eol in ('crlf', 'mixed') and not crlf_allowed:
            found.append(f'{path}: committed with CRLF line endings (index: {index_eol}); convert it to LF')
        elif worktree_eol in ('crlf', 'mixed') and index_eol == 'lf' and not crlf_allowed:
            found.append(f'{path}: checked out with CRLF; run: git rm --cached -r -q . && git reset -q --hard')
    return found


def main(argv=None):
    listing = subprocess.run(['git', 'ls-files', '--eol'], cwd=ROOT, capture_output=True, text=True, encoding='utf-8', check=True).stdout
    attributes_path = ROOT / '.gitattributes'
    attributes = attributes_path.read_text(encoding='utf-8') if attributes_path.is_file() else ''
    found = violations(listing, attributes)
    for line in found[:50]:
        print(line, file=sys.stderr)
    if len(found) > 50:
        print(f'... and {len(found) - 50} more', file=sys.stderr)
    print(f'line endings: {len(found)} problem(s)')
    return 1 if found else 0


if __name__ == '__main__':
    sys.exit(main(sys.argv[1:]))
