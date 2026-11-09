#!/usr/bin/env python3
"""Starts a Codex task in the checkout it runs in (docs/codex.md): a cloud container or the owner's PC.

    python tools/codex_start.py TASK_ID

Refuses unless the task is the one `codex_next.py` names. Fetches the original's Java source if it is missing, remembers the commit the work
starts from (`.git/codex-base`; the guard compares with it) and prints what the task is. Run it once, before any change.
"""
import json
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import codex_next  # noqa: E402

ROOT = codex_next.ROOT


def git(*args):
    result = subprocess.run(['git', *args], cwd=ROOT, capture_output=True, text=True, encoding='utf-8', errors='replace')
    if result.returncode != 0:
        sys.exit(f'FAILED: git {" ".join(args)}\n{result.stdout}{result.stderr}')
    return result.stdout.strip()


def main():
    if len(sys.argv) != 2:
        sys.exit(__doc__)
    task_id = sys.argv[1]
    _, nxt = codex_next.choose()
    if nxt != task_id:
        sys.exit(f'The next task is "{nxt}", not "{task_id}". Do the task codex_next.py names.')
    task = json.loads((codex_next.TASKS / f'{task_id}.json').read_text(encoding='utf-8'))
    if git('status', '--porcelain'):
        sys.exit('The tree has changes already. A task starts from a clean checkout of main.')
    result = subprocess.run([sys.executable, str(ROOT / 'tools/fetch_original.py')], cwd=ROOT)
    if result.returncode != 0:
        sys.exit('The original source could not be fetched (the container needs internet access for this one step).')
    base = git('rev-parse', 'HEAD')
    (Path(git('rev-parse', '--absolute-git-dir')) / 'codex-base').write_text(base + '\n', encoding='utf-8', newline='\n')
    print(f'TASK {task_id}: {task["title"]}  [{task.get("kind", "library")}]')
    print(f'STARTS FROM: {base[:10]}')
    print(f'JAVA (read only): {ROOT / ".local/tconstruct-source/src/main/java"}')
    print(f'The task file: docs/codex/tasks/{task_id}.json. When the work is done run:')
    print(f'    python tools/codex_finish.py {task_id} --final-message FILE')
    return 0


if __name__ == '__main__':
    sys.exit(main())
