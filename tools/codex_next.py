#!/usr/bin/env python3
"""Says which Codex task to do next, so Codex never chooses its own work (docs/codex.md).

    python tools/codex_next.py          # the table of tasks and the one to do
    python tools/codex_next.py --id     # only the id (empty when nothing is left)

A task is done when none of its inventory classes is `todo` in the checked-out tree, which is `main` when a run starts. The next task is the
lowest `order` whose dependencies are done. Nothing else may be started. The previous task's pull request must be merged before the next run.
"""
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
TASKS = ROOT / 'docs/codex/tasks'


def class_done(entry, task_id):
    """A class is done when it is no longer todo, and a class still partial counts only with the task's tag `[task-id]` in its note."""
    status = entry.get('status')
    if status == 'todo':
        return False
    if status == 'partial':
        return f'[{task_id}]' in entry.get('note', '')
    return True


def tasks():
    out = [json.loads(p.read_text(encoding='utf-8')) for p in sorted(TASKS.glob('*.json'))]
    return sorted(out, key=lambda t: (t.get('order', 999), t['id']))


def done_ids():
    """The tasks none of whose classes is todo in the tree."""
    done = set()
    cache = {}
    for t in tasks():
        inv = t.get('inventory')
        if not inv:
            continue
        data = cache.setdefault(inv['file'], json.loads((ROOT / inv['file']).read_text(encoding='utf-8')))
        if all(class_done(data['classes'].get(c, {}), t['id']) for c in inv['classes']):
            done.add(t['id'])
    return done


def choose():
    done = done_ids()
    table, nxt = [], None
    for t in tasks():
        waiting = [d for d in t.get('depends_on', []) if d not in done]
        state = 'done' if t['id'] in done else ('waits for ' + ', '.join(waiting) if waiting else 'ready')
        table.append((t.get('order', 0), t['id'], state, len(t['inventory']['classes']), t.get('kind', 'library')))
        if nxt is None and state == 'ready':
            nxt = t['id']
    return table, nxt


def main():
    table, nxt = choose()
    if '--id' in sys.argv:
        print(nxt or '')
        return 0 if nxt else 1
    for order, id_, state, classes, kind in table:
        print(f'{order:3}  {id_:40} {kind:8} {classes:3} classes  {state}')
    print()
    print(f'NEXT: {nxt}' if nxt else 'NEXT: nothing. Every task is done or waits for one that is not. Stop and write the final message; do not start other work.')
    return 0


if __name__ == '__main__':
    sys.exit(main())
