#!/usr/bin/env python3
"""Starts a Codex task in its own worktree (docs/codex.md).

    python tools/codex_start.py TASK_ID

Refuses unless the task is the one `codex_next.py` names. Makes `../codex-<id>` on a new branch `codex/<id>` from `origin/main`, links the original's
Java source into it (the library's data tests read it from `.local/tconstruct-source`) and prints the folder to work in.
"""
import json
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import codex_next  # noqa: E402

ROOT = codex_next.ROOT


def run(*cmd, cwd=ROOT):
    result = subprocess.run(cmd, cwd=cwd, capture_output=True, text=True, encoding='utf-8', errors='replace')
    if result.returncode != 0:
        sys.exit(f'FAILED: {" ".join(cmd)}\n{result.stdout}{result.stderr}')
    return result.stdout


def main():
    if len(sys.argv) != 2:
        sys.exit(__doc__)
    task_id = sys.argv[1]
    _, nxt = codex_next.choose()
    if nxt != task_id:
        sys.exit(f'The next task is "{nxt}", not "{task_id}". Do the task codex_next.py names.')
    task = json.loads((ROOT / 'docs/codex/tasks' / f'{task_id}.json').read_text(encoding='utf-8'))
    folder = ROOT.parent / f'codex-{task_id}'
    branch = f'codex/{task_id}'
    existing = run('git', 'branch', '--list', branch).strip()
    if folder.exists():
        print(f'The worktree already exists: {folder}')
    elif existing:
        run('git', 'worktree', 'add', str(folder), branch)
    else:
        run('git', 'worktree', 'add', str(folder), '-b', branch, 'origin/main')
    # the original's Java source and data are not in git: link them
    source = ROOT / '.local/tconstruct-source'
    link = folder / '.local/tconstruct-source'
    if source.exists() and not link.exists():
        (folder / '.local').mkdir(exist_ok=True)
        if sys.platform == 'win32':
            run('cmd', '/c', 'mklink', '/J', str(link), str(source))
        else:
            link.symlink_to(source, target_is_directory=True)
    print(f'TASK {task_id}: {task["title"]}')
    print(f'WORK IN: {folder}')
    print(f'BRANCH:  {branch}')
    print(f'JAVA:    {link}/src/main/java  (read only)')
    print()
    print('Before anything else set TMP, TEMP, CARGO_HOME and CARGO_TARGET_DIR to folders on the roomy drive (A:), outside the worktree.')
    print(f'When the work is done: python tools/codex_finish.py {task_id} --final-message FILE   (run from {folder})')
    return 0


if __name__ == '__main__':
    sys.exit(main())
