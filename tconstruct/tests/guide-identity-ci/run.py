#!/usr/bin/env python3
"""Two std-only production helper groups, not native Guide/Font/LAN validation."""
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import signal
import stat
import subprocess
import sys
import time

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
FLOOR = 10 * 1024**3 + 128 * 1024**2
BRANCH = 'checkpoint/guide-identity-ci-20261106'
REPOSITORY = 'sofia-gros/horizon-mods'
TOOLCHAIN = '1.95.0'
REQUIRED = {'runner', 'toolchain', 'compile', 'inventory', 'book_identity5', 'modifier_anchors5'}
RUNNER_TESTS = 12

def sha(data): return hashlib.sha256(data).hexdigest()
def git(*args): return subprocess.check_output(['git', '-C', str(ROOT), *args], text=True).strip()
def clean():
    if git('status', '--porcelain', '--untracked-files=all', '--ignored=matching'):
        raise ValueError('Dirty, ignored or untracked source')

def seal(root):
    result = {}
    for p in sorted(root.rglob('*')):
        rel = p.relative_to(root)
        if '.git' in rel.parts: continue
        mode = p.lstat().st_mode
        if stat.S_ISLNK(mode): value = ['symlink', os.readlink(p)]
        elif stat.S_ISDIR(mode): value = ['directory']
        elif stat.S_ISREG(mode): value = ['file', stat.S_IMODE(mode), sha(p.read_bytes())]
        else: raise ValueError('Special source entry: ' + str(p))
        result[rel.as_posix()] = value
    return result

def inputs():
    c = json.loads((HERE/'inputs.json').read_text())
    path = ROOT/'tconstruct/tests/guide-identity-proof/inputs.json'
    if sha(path.read_bytes()) != c['fixture_inputs_sha256']:
        raise ValueError('Frozen fixture inputs differ')
    f = json.loads(path.read_text())
    if (c['repository'], c['branch'], c['toolchain']) != (REPOSITORY, BRANCH, TOOLCHAIN):
        raise ValueError('Wrong repository/branch/toolchain')
    if f['source_sha256'] != c['source_sha256']:
        raise ValueError('Different source boundary')
    for name, digest in c['source_sha256'].items():
        p = ROOT/name
        if p.is_symlink() or sha(p.read_bytes()) != digest:
            raise ValueError('Frozen source differs: ' + name)
    groups = [(g['id'], g['prefix'], g['count'], g['filtered']) for g in f['groups']]
    if groups != [('book_identity5', 'identity::tests::', 5, 16),
                  ('modifier_anchors5', 'index::modifier_anchor_tests::', 5, 16)]:
        raise ValueError('Unbounded test groups')
    all_names = f['total_test_inventory']
    if len(all_names) != 21 or len(set(all_names)) != 21:
        raise ValueError('Inventory must contain 21 distinct names')
    for g in f['groups']:
        if len(g['names']) != 5 or set(g['names']) != {n for n in all_names if n.startswith(g['prefix'])}:
            raise ValueError('Incorrect selected names')
    return c, f

def validate_event(env, event, head):
    if (env.get('GITHUB_ACTIONS') != 'true' or env.get('GITHUB_REPOSITORY') != REPOSITORY or
        env.get('GITHUB_EVENT_NAME') != 'push' or env.get('GITHUB_REF') != 'refs/heads/'+BRANCH or
        env.get('GITHUB_SHA') != head or event.get('repository', {}).get('private') is not False or
        event.get('repository', {}).get('full_name') != REPOSITORY):
        raise ValueError('Require the exact authorized public push branch')

def existing(path):
    path = Path(path).expanduser().resolve()
    while not path.exists(): path = path.parent
    return path

def filesystems(work, env):
    return [existing(p) for p in [ROOT, work, '/', env.get('RUSTUP_HOME', str(Path.home()/'.rustup')),
                                 env.get('CARGO_HOME', str(Path.home()/'.cargo'))]]

def ensure_group_gone(pgid, name):
    deadline = time.monotonic()+1
    while True:
        try: os.killpg(pgid, 0)
        except ProcessLookupError: return
        if time.monotonic() >= deadline: raise RuntimeError('Own process group remains: '+name)
        time.sleep(0.02)

class Runner:
    def __init__(self, work, env):
        self.work, self.env, self.minimum = work, env, None
    def guard(self, reserve=0):
        free = min(shutil.disk_usage(p).free for p in filesystems(self.work, self.env))
        self.minimum = free if self.minimum is None else min(free, self.minimum)
        if free < FLOOR+reserve: raise RuntimeError('Disk floor reached')
    def run(self, name, command, timeout=120):
        self.guard()
        path = self.work/(name+'.log')
        print('PHASE', name, json.dumps(command), flush=True)
        started = time.monotonic()
        with path.open('x') as log:
            proc = subprocess.Popen(command, cwd=ROOT, env=self.env, stdout=log,
                                    stderr=subprocess.STDOUT, start_new_session=True)
            try:
                while proc.poll() is None:
                    self.guard()
                    if path.stat().st_size > 8*1024**2: raise RuntimeError('Output budget exceeded')
                    if time.monotonic()-started > timeout: raise TimeoutError(name)
                    time.sleep(0.2)
            finally:
                if proc.poll() is None:
                    try: os.killpg(proc.pid, signal.SIGTERM)
                    except ProcessLookupError: pass
                    deadline = time.monotonic()+1
                    while proc.poll() is None and time.monotonic() < deadline: time.sleep(0.05)
                try: os.killpg(proc.pid, signal.SIGKILL)
                except ProcessLookupError: pass
                proc.wait(timeout=2)
                ensure_group_gone(proc.pid, name)
                print(path.read_text(errors='replace'), end='', flush=True)
        self.guard()
        if path.stat().st_size > 8*1024**2: raise RuntimeError('Output budget exceeded')
        print('TERMINAL', name, proc.returncode, flush=True)
        if proc.returncode: raise RuntimeError(name+' exited '+str(proc.returncode))
        return path.read_text()

def validate_inventory(log, names):
    listed = re.findall(r'^(\S+): test$', log, re.M)
    if len(listed) != len(names) or set(listed) != set(names):
        raise ValueError('Missing, duplicate or extra inventory names')
    if re.findall(r'^(\d+) tests, (\d+) benchmarks$', log, re.M) != [('21', '0')]:
        raise ValueError('Wrong inventory summary')

def validate_tests(log, group):
    rows = re.findall(r'^test (\S+) \.\.\. (\w+)\s*$', log, re.M)
    if len(rows) != 5 or {n for n, _ in rows} != set(group['names']) or any(r != 'ok' for _, r in rows):
        raise ValueError('Missing, duplicate, extra or skipped test result')
    summaries = re.findall(r'^test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; (\d+) filtered out;', log, re.M)
    if summaries != [('5', '0', '0', '0', '16')]: raise ValueError('Wrong exact test summary')

def validate_python(log):
    if (re.findall(r'^Ran (\d+) tests in .+$', log, re.M) != [str(RUNNER_TESTS)] or
        len(re.findall(r'^test_\S+ .* \.\.\. ok$', log, re.M)) != RUNNER_TESTS or
        re.findall(r'^OK$', log, re.M) != ['OK']): raise ValueError('Runner checks did not all pass')

def binary_seal(path):
    mode = path.lstat().st_mode
    if not stat.S_ISREG(mode) or not mode & stat.S_IXUSR: raise ValueError('Missing compiler-produced executable')
    return {'sha256': sha(path.read_bytes()), 'size': path.stat().st_size, 'mode': stat.S_IMODE(mode)}

def error(report, exc):
    message = type(exc).__name__+': '+str(exc)
    report['error'] = (report['error']+'; ' if report['error'] else '')+message

def execute(report):
    validate_event(os.environ, json.loads(Path(os.environ['GITHUB_EVENT_PATH']).read_text()), git('rev-parse', 'HEAD'))
    clean(); c, f = inputs(); before = seal(ROOT)
    work = Path(os.environ['RUNNER_TEMP']).resolve()/('guide-identity-'+os.environ['GITHUB_SHA'])
    if ROOT == work or ROOT in work.parents: raise ValueError('Output must be outside source')
    env = os.environ.copy()
    for key in ['RUSTFLAGS', 'RUSTC_WRAPPER', 'RUSTC_WORKSPACE_WRAPPER', 'CARGO_ENCODED_RUSTFLAGS']:
        env.pop(key, None)
    env.update(PYTHONDONTWRITEBYTECODE='1', RUSTUP_TOOLCHAIN=TOOLCHAIN, TMPDIR=str(work))
    runner = Runner(work, env); runner.guard(reserve=2*1024**3)
    work.mkdir()  # Exclusive ownership; never reuse an earlier output directory.
    binary = work/'guide_identity_tests'; compiled = None
    try:
        validate_python(runner.run('runner-checks', [sys.executable, str(HERE/'test_runner.py'), '-v']))
        report['gates']['runner'] = RUNNER_TESTS
        runner.run('toolchain', ['rustup', 'toolchain', 'install', TOOLCHAIN, '--profile', 'minimal', '--no-self-update'], timeout=300)
        env['RUSTUP_AUTO_INSTALL'] = '0'
        compiler = Path(runner.run('compiler-path', ['rustup', 'which', '--toolchain', TOOLCHAIN, 'rustc']).strip())
        if not compiler.is_absolute() or not compiler.is_file(): raise ValueError('Invalid installed compiler path')
        report['compiler'] = str(compiler)
        report['gates']['toolchain'] = TOOLCHAIN
        runner.run('compile', [str(compiler), '--edition=2021', '--test', '--crate-name', 'guide_identity_tests',
                              '-C', 'debuginfo=0', str(ROOT/f['compile']['fixture']), '-o', str(binary)])
        compiled = binary_seal(binary); report['gates']['compile'] = compiled
        if seal(ROOT) != before: raise ValueError('Source drift after compile')
        validate_inventory(runner.run('inventory', [str(binary), '--list']), f['total_test_inventory'])
        if binary_seal(binary) != compiled: raise ValueError('Binary drift after inventory')
        report['gates']['inventory'] = 21
        for group in f['groups']:
            validate_tests(runner.run(group['id'], [str(binary), group['prefix'], '--test-threads=1']), group)
            if binary_seal(binary) != compiled: raise ValueError('Binary drift during execution')
            if seal(ROOT) != before: raise ValueError('Source drift during execution')
            report['gates'][group['id']] = {'passed': 5, 'filtered': 16, 'names': group['names']}
    except BaseException as exc: error(report, exc)
    finally:
        try:
            clean(); inputs()
            if seal(ROOT) != before: raise ValueError('Full source/file-set drift')
            if compiled is not None and binary_seal(binary) != compiled: raise ValueError('Final binary drift')
            runner.guard(); report['verified'] = True
        except BaseException as exc: error(report, exc)
        report.update(minimum_free_bytes=runner.minimum, full_source_seal_sha256=sha(json.dumps(before, sort_keys=True).encode()),
                      binary_seal=compiled, frozen_fixture=c['fixture_commit'])

def main():
    report = {'head': os.environ.get('GITHUB_SHA'), 'passed': False, 'verified': False, 'error': None,
              'gates': {}, 'floor_bytes': FLOOR,
              'scope': 'Exact std-only production BookId5 and modifier-anchor5. No native Text/Font/Game, installed consumer, translation refresh, screen lifetime, packets or LAN proof.'}
    try:
        execute(report)
        if set(report['gates']) != REQUIRED: raise ValueError('Missing required gate')
    except BaseException as exc: error(report, exc)
    report['passed'] = report['error'] is None and report['verified']
    try:
        with open(os.environ['GITHUB_STEP_SUMMARY'], 'a') as f:
            f.write('## Guide identity helper proof\n\n```json\n'+json.dumps(report, indent=2)+'\n```\n')
    except BaseException as exc: error(report, exc); report['passed'] = False
    print('FINAL_RECEIPT', json.dumps(report, sort_keys=True), flush=True)
    if not report['passed']: raise SystemExit(1)

if __name__ == '__main__': main()
