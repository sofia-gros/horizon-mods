#!/usr/bin/env python3
"""Skip expensive checks only when this PR's prior verified code is unchanged."""
import json
import os
from pathlib import Path
import subprocess

DOC_EXTENSIONS = {'.md', '.rst', '.png', '.jpg', '.jpeg', '.gif', '.svg', '.webp'}


def documentation_only(paths):
    return bool(paths) and all(
        path == 'README.md' or path.startswith('docs/') and Path(path).suffix.lower() in DOC_EXTENSIONS
        for path in paths
    )


def ancestor(older, newer):
    return subprocess.run(['git', 'merge-base', '--is-ancestor', older, newer], check=False).returncode == 0


def main():
    event = json.loads(Path(os.environ['GITHUB_EVENT_PATH']).read_text())
    pr = event.get('pull_request')
    if not pr or pr['head']['repo']['full_name'] != os.environ['GITHUB_REPOSITORY']:
        return False
    head, base = pr['head']['sha'], pr['base']['sha']
    result = subprocess.run(
        ['gh', 'api', '--method', 'GET', f"repos/{os.environ['GITHUB_REPOSITORY']}/actions/workflows/mixins.yml/runs",
         '-f', f"branch={pr['head']['ref']}", '-f', 'event=pull_request', '-f', 'status=success', '-f', 'per_page=100'],
        capture_output=True, text=True,
    )
    if result.returncode:
        print('Could not inspect prior successful runs; running full validation.')
        return False
    runs = json.loads(result.stdout)['workflow_runs']
    for run in runs:
        old = run['head_sha']
        if not any(item['number'] == pr['number'] for item in run.get('pull_requests', [])):
            continue
        if not ancestor(base, old) or not ancestor(old, head):
            continue
        diff = subprocess.run(['git', 'diff', '--no-renames', '--name-only', '-z', old, head], capture_output=True, check=True).stdout
        paths = [path.decode() for path in diff.split(b'\0') if path]
        if documentation_only(paths):
            print(f"Documentation-only changes since successful run {run['id']} ({old}).")
            return True
    print('No applicable successful ancestor run; running full validation.')
    return False


if __name__ == '__main__':
    skip = main()
    with open(os.environ['GITHUB_OUTPUT'], 'a') as output:
        output.write(f"docs_only={'true' if skip else 'false'}\n")
