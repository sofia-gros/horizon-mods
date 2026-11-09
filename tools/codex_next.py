#!/usr/bin/env python3
"""Says which Codex task to do next, so Codex never chooses its own work (docs/codex.md).

    python tools/codex_next.py          # the table of tasks and the one to do
    python tools/codex_next.py --id     # only the id (empty when nothing is left)

A task is done when none of its inventory classes is `todo` on `origin/main`. The next task is the one in progress (a local branch `codex/<id>` that is
not merged), else the lowest `order` whose dependencies are done. Nothing else may be started.
"""
import json
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import codex_guard as guard  # noqa: E402

ROOT = guard.ROOT


def tasks():
    out = []
    for path in sorted((ROOT / 'docs/codex/tasks').glob('*.json')):
        out.append(json.loads(path.read_text(encoding='utf-8')))
    return sorted(out, key=lambda t: (t.get('order', 999), t['id']))


def fetch():
    subprocess.run(['git', 'fetch', '-q', 'origin'], cwd=ROOT, capture_output=True)


def open_branches(base='origin/main'):
    rows = subprocess.run(['git', 'branch', '--list', 'codex/*', '--format=%(refname:short)'], cwd=ROOT, capture_output=True, text=True).stdout.split()
    merged = subprocess.run(['git', 'branch', '--list', 'codex/*', '--merged', base, '--format=%(refname:short)'], cwd=ROOT, capture_output=True, text=True).stdout.split()
    return [b[len('codex/'):] for b in rows if b not in merged]


def choose(base='origin/main'):
    fetch()
    done = guard.merged_tasks(base)
    in_progress = [t for t in open_branches(base) if t not in done]
    table = []
    nxt = None
    for t in tasks():
        waiting = [d for d in t.get('depends_on', []) if d not in done]
        if t['id'] in done:
            state = 'done'
        elif t['id'] in in_progress:
            state = 'IN PROGRESS'
        elif waiting:
            state = 'waits for ' + ', '.join(waiting)
        else:
            state = 'ready'
        table.append((t.get('order', 0), t['id'], state, len(t['inventory']['classes'])))
    for t in tasks():
        if t['id'] in in_progress:
            nxt = t['id']
            break
    if nxt is None:
        for order, id_, state, _ in table:
            if state == 'ready':
                nxt = id_
                break
    return table, nxt


def main():
    table, nxt = choose()
    if '--id' in sys.argv:
        print(nxt or '')
        return 0 if nxt else 1
    for order, id_, state, classes in table:
        print(f'{order:3}  {id_:38} {classes:3} classes  {state}')
    print()
    print(f'NEXT: {nxt}' if nxt else 'NEXT: nothing. Every task is done or waits for one that is not. Stop and write the final message; do not start other work.')
    return 0


if __name__ == '__main__':
    sys.exit(main())
