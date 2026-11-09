#!/usr/bin/env python3
"""Finishes a Codex task (docs/codex.md): runs every gate and, only if all pass, publishes the branch.

    python tools/codex_finish.py TASK_ID --final-message FILE [--dry-run]    (run inside the task's worktree)

The gates, in this order, each must pass:
  1. the branch is `codex/<id>` and everything is committed;
  2. `tools/codex_guard.py <id>` prints `0 problem(s)`;
  3. every command under `verify` in the task exits 0 (their output is kept in the pull request);
  4. the final message file has the required shape (docs/codex.md).
Then, for a task with `"auto_merge": true`, the branch is pushed, a pull request is opened with the final message as its body and merged
(merge commit), and the worktree is left for the next task. A task without auto_merge is pushed and opened as a draft pull request for the
owner's agent to check in game, and is NOT merged. Codex may never merge by any other way.
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


def sh(command, cwd=ROOT, check=True):
    result = subprocess.run(command, cwd=cwd, shell=isinstance(command, str), capture_output=True, text=True, encoding='utf-8', errors='replace')
    if check and result.returncode != 0:
        print(result.stdout[-3000:], result.stderr[-3000:])
        sys.exit(f'GATE FAILED: {command if isinstance(command, str) else " ".join(command)}')
    return result


def verify_commands(task):
    """The verify entries that are commands (an entry may end with a remark in parentheses, which is dropped)."""
    commands = []
    for entry in task['verify']:
        command = re.sub(r'\s*\(.*\)\s*$', '', entry).strip()
        if command.startswith(('python ', 'cargo ')):
            commands.append(command.replace('python ', f'"{sys.executable}" ', 1) if command.startswith('python ') else command)
    return commands


def main():
    args = sys.argv[1:]
    if not args or '--final-message' not in args:
        sys.exit(__doc__)
    task_id = args[0]
    dry = '--dry-run' in args
    message = Path(args[args.index('--final-message') + 1]).read_text(encoding='utf-8')
    task = json.loads((ROOT / 'docs/codex/tasks' / f'{task_id}.json').read_text(encoding='utf-8'))

    sh(['git', 'fetch', '-q', 'origin'])
    branch = sh(['git', 'rev-parse', '--abbrev-ref', 'HEAD']).stdout.strip()
    if branch != f'codex/{task_id}':
        sys.exit(f'GATE FAILED: on branch {branch}, expected codex/{task_id}')
    if sh(['git', 'status', '--porcelain']).stdout.strip():
        sys.exit('GATE FAILED: uncommitted changes; commit everything first (commit messages written to a file, git commit -F)')
    for key in SHAPE:
        if key not in message:
            sys.exit(f'GATE FAILED: the final message lacks "{key}" (the shape is in docs/codex.md)')
    if 'NOT RUN' in message.split('VERIFY:')[1].split('QUESTIONS:')[0] and any(c.startswith('cargo') for c in verify_commands(task)):
        sys.exit('GATE FAILED: the VERIFY part says NOT RUN for a command that finish runs itself; run it and copy the result')

    guard_run = sh([sys.executable, str(ROOT / 'tools/codex_guard.py'), task_id, '--base', 'origin/main'], check=False)
    print(guard_run.stdout)
    if guard_run.returncode != 0:
        sys.exit('GATE FAILED: the guard found problems (above). Fix them in the branch and run finish again.')

    results = []
    for command in verify_commands(task):
        print(f'$ {command}')
        run = sh(command, check=False)
        tail = (run.stdout + run.stderr).strip().splitlines()[-6:]
        results.append((command, run.returncode, '\n'.join(tail)))
        if run.returncode != 0:
            print('\n'.join(tail))
            sys.exit(f'GATE FAILED: {command} exited {run.returncode}')
    print('all gates passed')
    if dry:
        print('dry run: nothing pushed')
        return 0

    body = message.strip() + '\n\n## Gates run by codex_finish.py\n' + '\n'.join(f'- `{c}`: exit {rc}' for c, rc, _ in results) + '\n\nOpened by Codex. The game was not run.\n'
    body_file = ROOT / '.git' / f'codex-{task_id}-pr.md'
    body_file.write_text(body, encoding='utf-8')
    sh(['git', 'push', '-u', 'origin', branch])
    auto = bool(task.get('auto_merge'))
    create = ['gh', 'pr', 'create', '--title', f'Codex: {task["title"]}', '--body-file', str(body_file)] + ([] if auto else ['--draft'])
    url = sh(create).stdout.strip().splitlines()[-1]
    print(url)
    if auto:
        sh(['gh', 'pr', 'merge', url, '--merge'])
        print('merged')
        sh(['git', 'fetch', '-q', 'origin'])
    else:
        print('left as a draft: the owner\'s agent checks it in game before it is merged')
    return 0


if __name__ == '__main__':
    sys.exit(main())
