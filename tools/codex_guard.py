#!/usr/bin/env python3
r"""Checks that a Codex branch stayed inside its task (docs/codex.md, docs/codex/tasks/<id>.json).

Codex works on a branch `codex/<id>` and never merges. Before the owner's agent reviews it, this script compares the branch with its base
and fails when it:
- touches a path the task does not allow, or one that is always forbidden (AGENTS.md, the roadmap, workflows, tools, the other products);
- adds a test, a Markdown file or a binary the task did not ask for;
- changes more files or lines than the task's limits;
- writes CRLF line endings;
- changes the inventory beyond the classes the task names, or marks one `ported` (only the owner's agent may after a check in game).

Usage: codex_guard.py TASK_ID [--base main]
"""
import argparse
import json
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
ALWAYS_FORBIDDEN = ['AGENTS.md', 'CLAUDE.md', 'docs/roadmap.md', 'docs/codex.md', 'docs/codex/**', '.github/**', 'tools/**', 'tests/**', '.gitattributes']
BINARY_SUFFIXES = ('.png', '.jar', '.zip', '.exe', '.dll', '.ogg', '.bin')


def pattern(glob):
    """`**` crosses folders, `*` does not."""
    out = ''
    i = 0
    while i < len(glob):
        if glob[i:i + 3] == '**/':
            out += '(?:.*/)?'
            i += 3
        elif glob[i:i + 2] == '**':
            out += '.*'
            i += 2
        elif glob[i] == '*':
            out += '[^/]*'
            i += 1
        else:
            out += re.escape(glob[i])
            i += 1
    return re.compile('^' + out + '$')


def matches(path, globs):
    return any(pattern(g).match(path) for g in globs)


def violations(task, changes, inventory_before=None, inventory_after=None):
    """changes: [{path, status ('A', 'M', 'D'), added, removed, crlf}]; returns the list of problems."""
    found = []
    allowed = task.get('allowed_paths', [])
    explicit = [g for g in allowed if not g.startswith('**')]
    total = 0
    for change in changes:
        path = change['path']
        total += change['added'] + change['removed']
        if matches(path, ALWAYS_FORBIDDEN) and not matches(path, [g for g in explicit if g != '**']):
            found.append(f'{path}: always off limits for Codex')
        elif not matches(path, allowed):
            found.append(f'{path}: not in the task\'s allowed_paths')
        if change['status'] == 'D' and not matches(path, task.get('may_delete', [])):
            found.append(f'{path}: deleted, but the task does not list it in may_delete')
        if change['status'] == 'A':
            if re.search(r'(^|/)(tests?|test_[^/]*|[^/]*_test\.rs)(/|$)', path) and not task.get('tests_allowed'):
                found.append(f'{path}: a new test; this task asks for none')
            if path.endswith('.md') and not task.get('markdown_allowed'):
                found.append(f'{path}: a new Markdown file; this task asks for none')
            if path.lower().endswith(BINARY_SUFFIXES) and not task.get('binary_allowed'):
                found.append(f'{path}: a new binary file; this task asks for none')
        if change['crlf']:
            found.append(f'{path}: CRLF line endings (LF everywhere)')
    if len(changes) > task.get('max_files', 50):
        found.append(f'{len(changes)} files changed, the task allows {task.get("max_files", 50)}')
    if total > task.get('max_changed_lines', 4000):
        found.append(f'{total} lines changed, the task allows {task.get("max_changed_lines", 4000)}')
    inv = task.get('inventory')
    if inv and inventory_before is not None and inventory_after is not None:
        names = set(inv['classes'])
        for name, entry in inventory_after['classes'].items():
            before = inventory_before['classes'].get(name)
            if before != entry:
                if name not in names:
                    found.append(f'inventory: {name} changed but the task does not name it')
                elif entry['status'] == 'ported':
                    found.append(f'inventory: {name} marked ported; Codex may write at most ported-unchecked (the owner\'s agent checks in game)')
                elif not entry.get('note'):
                    found.append(f'inventory: {name} needs a note')
    return found


def git(*args):
    return subprocess.run(['git', *args], cwd=ROOT, capture_output=True, text=True, encoding='utf-8', check=True).stdout


def collect(base):
    changes = []
    rows = git('diff', '--name-status', '--no-renames', f'{base}...HEAD').splitlines()
    numstat = {}
    for line in git('diff', '--numstat', '--no-renames', f'{base}...HEAD').splitlines():
        added, removed, path = line.split('\t', 2)
        numstat[path] = (0 if added == '-' else int(added), 0 if removed == '-' else int(removed))
    for row in rows:
        status, path = row.split('\t', 1)
        crlf = False
        if status != 'D' and (ROOT / path).is_file():
            try:
                crlf = b'\r\n' in (ROOT / path).read_bytes()
            except OSError:
                pass
        added, removed = numstat.get(path, (0, 0))
        changes.append({'path': path, 'status': status[0], 'added': added, 'removed': removed, 'crlf': crlf})
    return changes


def main():
    p = argparse.ArgumentParser()
    p.add_argument('task')
    p.add_argument('--base', default='main')
    a = p.parse_args()
    task = json.loads((ROOT / 'docs/codex/tasks' / f'{a.task}.json').read_text(encoding='utf-8'))
    before = after = None
    inv = task.get('inventory')
    if inv:
        before = json.loads(git('show', f'{a.base}:{inv["file"]}'))
        after = json.loads((ROOT / inv['file']).read_text(encoding='utf-8'))
    found = violations(task, collect(a.base), before, after)
    for line in found:
        print(line)
    print(f'codex guard: {len(found)} problem(s)')
    return 1 if found else 0


if __name__ == '__main__':
    sys.exit(main())
