#!/usr/bin/env python3
"""Runs a Codex pull request on the owner's PC, with the game, and merges it if everything passes (docs/codex.md).

    python tools/codex_pc_check.py PR_NUMBER [--merge] [--no-game]

Needs `gh` logged in and, for the game part, the same setup `tools/smoke-local/probe_pluto_lib.py` needs (the launcher, the prepared resources).
Steps: check out the pull request; run the guard for its task (the task id is the `TASK <id>` line of the PR description); the library tests;
the documents and line-ending checks; if the change touches the Tinkers API crate, PlutoLib or the probe: the game probe. Prints a summary.
With `--merge` it merges the pull request when every step passed. It always goes back to `main` at the end.
"""
import json
import os
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
PY = sys.executable
LAUNCHER = ROOT / '.local/cargo-target-launcher/release/horizon-launcher.exe'
RESOURCES = ROOT / '.local/smoke-build/game-resources'


def run(cmd, env=None, cwd=ROOT, quiet=False):
    result = subprocess.run(cmd, cwd=cwd, env=env, capture_output=True, text=True, encoding='utf-8', errors='replace')
    if not quiet and result.returncode != 0:
        print((result.stdout + result.stderr)[-1500:])
    return result


def game_env():
    env = dict(os.environ)
    local = ROOT / '.local'
    env.update(HORIZON_LAUNCHER_HOME=str(local / 'launcher-home'), HORIZON_HOME=str(local / 'horizon-home'), XDG_DATA_HOME=str(local / 'xdg-data'),
               XDG_CONFIG_HOME=str(local / 'xdg-config'), TMP=str(local / 'tmp'), TEMP=str(local / 'tmp'), CARGO_HOME=str(local / 'launcher-cargo-home'))
    return env


def main():
    args = sys.argv[1:]
    if not args or not args[0].isdigit():
        sys.exit(__doc__)
    number = args[0]
    merge, game = '--merge' in args, '--no-game' not in args
    if run(['git', 'status', '--porcelain'], quiet=True).stdout.strip():
        sys.exit('The checkout has changes. Commit or stash them first.')
    info = json.loads(run(['gh', 'pr', 'view', number, '--json', 'body,title,baseRefName'], quiet=True).stdout)
    match = re.search(r'^TASK (\S+)', info['body'], re.M)
    if not match:
        sys.exit('The pull request description has no "TASK <id>" line (the final message of the task).')
    task_id = match.group(1)
    steps = []
    try:
        if run(['gh', 'pr', 'checkout', number]).returncode != 0:
            sys.exit('gh pr checkout failed')
        run(['git', 'fetch', '-q', 'origin'], quiet=True)
        # the guard compares with the commit the work started from: the merge base with main
        base = run(['git', 'merge-base', 'HEAD', 'origin/main'], quiet=True).stdout.strip()
        steps.append(('guard', run([PY, 'tools/codex_guard.py', task_id, '--base', base]).returncode == 0))
        steps.append(('library tests', run(['cargo', 'test', '--locked', '--manifest-path', 'tconstruct/mixins/tconstruct/api/library/Cargo.toml']).returncode == 0))
        steps.append(('documents', run([PY, 'tools/check_docs_budget.py']).returncode == 0))
        steps.append(('line endings', run([PY, 'tools/check_line_endings.py']).returncode == 0))
        changed = run(['git', 'diff', '--name-only', base, 'HEAD'], quiet=True).stdout.split()
        if game and any(p.startswith(('tconstruct/mixins/tconstruct/api/src/', 'pluto-lib/', 'tconstruct/content/')) or p == 'tools/smoke-local/probe_pluto_lib.py' for p in changed):
            print('the change touches the game side: running the game probe (about 10 minutes) ...')
            probe = run([PY, 'tools/smoke-local/probe_pluto_lib.py', '--launcher', str(LAUNCHER), '--resources', str(RESOURCES)], env=game_env(), quiet=True)
            text = probe.stdout + probe.stderr
            steps.append(('game probe', probe.returncode == 0 and text.lstrip().startswith('PROBE PASS')))
            if not steps[-1][1]:
                print(text[:1500])
        else:
            steps.append(('game probe', None))
    finally:
        run(['git', 'checkout', '-q', 'main'], quiet=True)
    print()
    for name, ok in steps:
        print(f'  {"PASS" if ok else ("skipped (nothing on the game side)" if ok is None else "FAIL")}  {name}')
    passed = all(ok is not False for _, ok in steps)
    print()
    if passed and merge:
        merged = run(['gh', 'pr', 'merge', number, '--merge'])
        print('merged' if merged.returncode == 0 else 'the merge failed (above)')
        run(['git', 'pull', '-q'], quiet=True)
    elif passed:
        print(f'everything passed; to merge: python tools/codex_pc_check.py {number} --merge   (or merge it on GitHub)')
    else:
        print('something failed. Give the lines above to Codex as the follow-up of this task; do not merge.')
    return 0 if passed else 1


if __name__ == '__main__':
    sys.exit(main())
