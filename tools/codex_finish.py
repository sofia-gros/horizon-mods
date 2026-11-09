#!/usr/bin/env python3
"""Checks a finished Codex task (docs/codex.md): every gate must pass before the work is handed over.

    python tools/codex_finish.py TASK_ID --final-message FILE

The gates, in this order, each must pass:
  1. the final message file has the required shape (docs/codex.md);
  2. `tools/codex_guard.py <id>` prints `0 problem(s)` (it stages the changes to read them; commit them afterwards);
  3. every command under `verify` in the task exits 0.
It prints `READY` only when all pass. The work is then handed over as the pull request of the cloud run (or, on the owner's PC, by
`tools/codex_pc_check.py`); nothing is pushed or merged from here.
"""
import json
import re
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import codex_guard as guard  # noqa: E402

ROOT = guard.ROOT
SHAPE = ['TASK ', 'DONE:', 'PARTIAL / LEFT:', 'FILES:', 'VERIFY:', 'TESTS:', 'QUESTIONS:']


def sh(command, check=True):
    result = subprocess.run(command, cwd=ROOT, shell=isinstance(command, str), capture_output=True, text=True, encoding='utf-8', errors='replace')
    if check and result.returncode != 0:
        print(result.stdout[-3000:], result.stderr[-3000:])
        sys.exit(f'GATE FAILED: {command if isinstance(command, str) else " ".join(command)}')
    return result


def verify_commands(task):
    """The verify entries that are commands (an entry may end with a remark in parentheses, which is dropped)."""
    commands = []
    for entry in task['verify']:
        command = re.sub(r'\s*\(.*\)\s*$', '', entry).strip()
        if command.startswith('python '):
            commands.append(f'"{sys.executable}" ' + command[len('python '):])
        elif command.startswith('cargo '):
            commands.append(command)
    return commands


def main():
    args = sys.argv[1:]
    if not args or '--final-message' not in args:
        sys.exit(__doc__)
    task_id = args[0]
    message = Path(args[args.index('--final-message') + 1]).read_text(encoding='utf-8')
    task = json.loads((ROOT / 'docs/codex/tasks' / f'{task_id}.json').read_text(encoding='utf-8'))

    for key in SHAPE:
        if key not in message:
            sys.exit(f'GATE FAILED: the final message lacks "{key}" (the shape is in docs/codex.md)')
    verify_part = message.split('VERIFY:')[1].split('TESTS:')[0]
    if 'NOT RUN' in verify_part and any(c.startswith('cargo') for c in verify_commands(task)):
        sys.exit('GATE FAILED: the VERIFY part says NOT RUN for a command that finish runs itself; run it and copy the result')

    guard_run = sh([sys.executable, str(ROOT / 'tools/codex_guard.py'), task_id], check=False)
    print(guard_run.stdout)
    if guard_run.returncode != 0:
        sys.exit('GATE FAILED: the guard found problems (above). Fix them and run finish again.')

    for command in verify_commands(task):
        print(f'$ {command}')
        run = sh(command, check=False)
        tail = (run.stdout + run.stderr).strip().splitlines()[-6:]
        if run.returncode != 0:
            print('\n'.join(tail))
            sys.exit(f'GATE FAILED: {command} exited {run.returncode}')
    print('READY: all gates passed. Commit the work; the owner runs it on the PC before it is merged.')
    return 0


if __name__ == '__main__':
    sys.exit(main())
