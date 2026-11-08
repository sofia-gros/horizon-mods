#!/usr/bin/env python3
r"""The feature inventory of a domain: every class of the original Java package and where its port stands.

A phase of the roadmap is complete only when every class in `docs/inventory/<domain>.json` is `ported` (checked in game), `not-code`
or `parked` with a reason. `todo`, `partial` and `ported-unchecked` (written by Codex, not yet checked in game) keep the phase incomplete; a class of the original that is not in the file is an error,
so a feature cannot be forgotten by not listing it.

Usage: feature_inventory.py [domain ...]   (default: every file in docs/inventory); --strict exits 1 while a phase is incomplete.
"""
import json
import sys
from collections import Counter
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
JAVA = ROOT / '.local/tconstruct-source/src/main/java'
STATUSES = ('ported', 'ported-unchecked', 'not-code', 'parked', 'partial', 'todo')


def report(path):
    data = json.loads(path.read_text(encoding='utf-8'))
    classes = data['classes']
    problems = []
    for name, entry in classes.items():
        if entry['status'] not in STATUSES:
            problems.append(f'{name}: unknown status {entry["status"]}')
        if entry['status'] in ('parked', 'todo', 'partial', 'not-code', 'ported-unchecked') and not entry.get('note'):
            problems.append(f'{name}: a {entry["status"]} class needs a note')
    package = JAVA / data['java']
    if package.exists():
        found = {p.relative_to(package).as_posix() for p in package.rglob('*.java') if p.name != 'package-info.java'}
        problems += [f'{n}: in the original but not in the inventory' for n in sorted(found - set(classes))]
        problems += [f'{n}: in the inventory but not in the original' for n in sorted(set(classes) - found)]
    count = Counter(e['status'] for e in classes.values())
    open_ = count['todo'] + count['partial'] + count['ported-unchecked']
    return data['domain'], count, open_, problems


def main(argv):
    strict = '--strict' in argv
    names = [a for a in argv if not a.startswith('--')]
    files = [ROOT / 'docs/inventory' / f'{n}.json' for n in names] or sorted((ROOT / 'docs/inventory').glob('*.json'))
    failed = incomplete = False
    for path in files:
        domain, count, open_, problems = report(path)
        summary = ', '.join(f'{count[s]} {s}' for s in STATUSES if count[s])
        print(f'{domain}: {"INCOMPLETE" if open_ else "complete"} ({summary})')
        for line in problems:
            print('  ' + line)
        failed |= bool(problems)
        incomplete |= bool(open_)
    return 1 if failed or (strict and incomplete) else 0


if __name__ == '__main__':
    sys.exit(main(sys.argv[1:]))
