#!/usr/bin/env python3
r"""Checks that a Codex branch stayed inside its task (docs/codex.md, docs/codex/tasks/<id>.json).

Codex works on a branch `codex/<id>` and never merges. Before the owner's agent reviews it, this script compares the branch with its base
and fails when it:
- touches a path the task does not allow, or one that is always forbidden (AGENTS.md, the roadmap, workflows, tools, the other products);
- adds a test, a Markdown file or a binary the task did not ask for;
- changes more files or lines than the task's limits;
- writes CRLF line endings;
- adds more tests than the task allows, removes or ignores a test, or writes a test that cannot fail (`assert!(true)`, comparing a value
  with itself);
- adds placeholder code (`todo!`, `unimplemented!`, `#[allow(...)]`, `dbg!`, `println!` outside a binary) or turns a code file into mostly comments;
- changes a build file (`Cargo.toml`, `Cargo.lock`, `rust-toolchain`) the task does not list;
- starts before the tasks it depends on are merged (their inventory classes must no longer be `todo` on the base), or on a branch that is not
  `codex/<id>`;
- changes the inventory beyond the classes the task names, marks one `ported`, or writes a note that says nothing (too short, no mention of a test);

`--lint` checks every task file instead of a branch (required fields, that the files it names exist, that its classes are `todo` in the inventory).

Usage: codex_guard.py TASK_ID [--base origin/main]    |    codex_guard.py --lint
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


BUILD_FILES = re.compile(r'(^|/)(Cargo\.toml|Cargo\.lock|rust-toolchain(\.toml)?|\.cargo/.*|build\.rs)$')
PLACEHOLDERS = re.compile(r'\b(todo!|unimplemented!|dbg!)\s*\(|#!?\[allow\(|#\[ignore|println!\s*\(|eprintln!\s*\(')
VACUOUS = re.compile(r'assert!\(\s*true\s*\)|assert_eq!\(\s*(\w+)\s*,\s*\1\s*\)|assert_ne!\(\s*(\w+)\s*,\s*\2\s*\)|assert!\(\s*!\s*false\s*\)')
TEST_ATTR = re.compile(r'^\s*#\[test\]')
ASSERTION = re.compile(r'\bassert(?:_eq|_ne)?!\(|\bpanic!\(|\.unwrap_err\(\)|\bassert_matches!\(')
MAX_COMMENT_RATIO = 0.35


def diff_lines(base):
    """{path: (added_lines, removed_lines)} of the text of the diff."""
    out, path = {}, None
    for line in git('diff', '--no-renames', '-U0', f'{base}...HEAD').splitlines():
        if line.startswith('+++ b/'):
            path = line[6:]
            out.setdefault(path, ([], []))
        elif line.startswith('--- ') or line.startswith('+++ '):
            continue
        elif path and line.startswith('+'):
            out[path][0].append(line[1:])
        elif path and line.startswith('-'):
            out[path][1].append(line[1:])
    return out


def text_violations(task, texts):
    """texts: {path: (added_lines, removed_lines)}; the rules about what the new code looks like."""
    found = []
    new_tests = 0
    for path, (added, removed) in texts.items():
        if not path.endswith('.rs'):
            continue
        new_tests += sum(1 for l in added if TEST_ATTR.match(l))
        gone = sum(1 for l in removed if TEST_ATTR.match(l))
        if gone > sum(1 for l in added if TEST_ATTR.match(l)):
            found.append(f'{path}: removes a test; this task does not ask for it')
        tests_here = sum(1 for l in added if TEST_ATTR.match(l))
        asserts_here = sum(len(ASSERTION.findall(l)) for l in added)
        if tests_here and asserts_here < tests_here:
            found.append(f'{path}: {tests_here} new tests but only {asserts_here} assertions; every test must assert something')
        for l in added:
            if PLACEHOLDERS.search(l):
                found.append(f'{path}: placeholder or debugging code: {l.strip()[:80]}')
            if VACUOUS.search(l):
                found.append(f'{path}: a test that cannot fail: {l.strip()[:80]}')
        code = [l for l in added if l.strip() and not l.strip().startswith('//')]
        comments = [l for l in added if l.strip().startswith('//')]
        if len(added) >= 40 and len(comments) / max(1, len(added)) > MAX_COMMENT_RATIO:
            found.append(f'{path}: {len(comments)} of {len(added)} new lines are comments (at most {int(MAX_COMMENT_RATIO * 100)}%); write code, not prose')
        if len(added) >= 40 and len(code) < len(added) * 0.5:
            found.append(f'{path}: less than half of the new lines are code')
    limit = task.get('max_new_tests')
    if limit is not None and new_tests > limit:
        found.append(f'{new_tests} new tests, the task allows {limit} (one test per behaviour, not per line)')
    return found


def violations(task, changes, inventory_before=None, inventory_after=None, texts=None, base_inventory_by_task=None, branch=None):
    """changes: [{path, status ('A', 'M', 'D'), added, removed, crlf}]; returns the list of problems."""
    found = []
    if branch is not None and branch != f'codex/{task["id"]}':
        found.append(f'branch is {branch}; the work belongs on codex/{task["id"]}')
    for dep in task.get('depends_on', []):
        if base_inventory_by_task is not None and dep not in base_inventory_by_task:
            found.append(f'depends on {dep}, which is not merged into the base yet; start it after the owner\'s agent merges {dep}')
    if texts is not None:
        found += text_violations(task, texts)
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
        if BUILD_FILES.search(path) and path not in task.get('build_files_allowed', []):
            found.append(f'{path}: a build file; the task does not allow changing it')
    added_files = sum(1 for c in changes if c['status'] == 'A')
    if added_files > task.get('max_new_files', 30):
        found.append(f'{added_files} new files, the task allows {task.get("max_new_files", 30)}')
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
                elif entry['status'] in ('ported-unchecked', 'partial') and (len(entry['note']) < 40 or 'test' not in entry['note'].lower()):
                    found.append(f'inventory: the note of {name} must say where the port is and what is tested (at least 40 characters, mention the test)')
                elif entry['status'] not in ('ported-unchecked', 'partial', 'not-code', 'todo'):
                    found.append(f'inventory: {name} has the status {entry["status"]}; Codex may write ported-unchecked, partial, not-code or leave todo')
        for name in inventory_before['classes']:
            if name not in inventory_after['classes']:
                found.append(f'inventory: {name} was removed')
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


REQUIRED = ['id', 'title', 'goal', 'read_first', 'java', 'inventory', 'allowed_paths', 'max_files', 'max_changed_lines', 'max_new_tests', 'verify', 'stop_and_report_if', 'notes']


def lint():
    """Every task file: the fields are there, the files it names exist, its classes are todo, no two tasks share a class."""
    problems = []
    seen = {}
    ids = {p.stem for p in (ROOT / 'docs/codex/tasks').glob('*.json')}
    for path in sorted((ROOT / 'docs/codex/tasks').glob('*.json')):
        task = json.loads(path.read_text(encoding='utf-8'))
        name = path.name
        for key in REQUIRED:
            if key not in task:
                problems.append(f'{name}: missing "{key}"')
        if task.get('id') != path.stem:
            problems.append(f'{name}: "id" must be {path.stem}')
        for rel in task.get('read_first', []):
            if not (ROOT / rel).exists():
                problems.append(f'{name}: read_first {rel} does not exist')
        for dep in task.get('depends_on', []):
            if dep not in ids:
                problems.append(f'{name}: depends_on {dep}, which is not a task')
        inv = task.get('inventory')
        if inv:
            data = json.loads((ROOT / inv['file']).read_text(encoding='utf-8'))
            for cls in inv['classes']:
                entry = data['classes'].get(cls)
                if entry is None:
                    problems.append(f'{name}: class {cls} is not in {inv["file"]}')
                elif entry['status'] not in ('todo', 'partial'):
                    problems.append(f'{name}: class {cls} is {entry["status"]}, a task needs todo or partial')
                if cls in seen and seen[cls] != name:
                    problems.append(f'{name}: class {cls} is also in {seen[cls]}')
                seen[cls] = name
            java_root = ROOT / '.local/tconstruct-source/src/main/java'
            if java_root.is_dir():
                for rel in task.get('java', []):
                    if not (java_root / rel).exists():
                        problems.append(f'{name}: java {rel} does not exist in the original\'s source')
        if task.get('max_new_tests', 0) < 1:
            problems.append(f'{name}: max_new_tests must be set (a number above 0)')
    for line in problems:
        print(line)
    print(f'codex guard lint: {len(problems)} problem(s) in {len(ids)} task file(s)')
    return 1 if problems else 0


def merged_tasks(base):
    """The ids of tasks whose classes are no longer todo on the base, i.e. merged."""
    done = set()
    for path in (ROOT / 'docs/codex/tasks').glob('*.json'):
        task = json.loads(path.read_text(encoding='utf-8'))
        inv = task.get('inventory')
        if not inv:
            done.add(path.stem)
            continue
        data = json.loads(git('show', f'{base}:{inv["file"]}'))
        if all(data['classes'].get(c, {}).get('status') != 'todo' for c in inv['classes']):
            done.add(path.stem)
    return done


def main():
    p = argparse.ArgumentParser()
    p.add_argument('task', nargs='?')
    p.add_argument('--base', default='origin/main')
    p.add_argument('--lint', action='store_true')
    a = p.parse_args()
    if a.lint:
        return lint()
    if not a.task:
        p.error('a task id, or --lint')
    task = json.loads((ROOT / 'docs/codex/tasks' / f'{a.task}.json').read_text(encoding='utf-8'))
    before = after = None
    inv = task.get('inventory')
    if inv:
        before = json.loads(git('show', f'{a.base}:{inv["file"]}'))
        after = json.loads((ROOT / inv['file']).read_text(encoding='utf-8'))
    branch = git('rev-parse', '--abbrev-ref', 'HEAD').strip()
    found = violations(task, collect(a.base), before, after, diff_lines(a.base), merged_tasks(a.base), branch)
    for line in found:
        print(line)
    print(f'codex guard: {len(found)} problem(s)')
    return 1 if found else 0


if __name__ == '__main__':
    sys.exit(main())
