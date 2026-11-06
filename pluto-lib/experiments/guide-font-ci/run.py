#!/usr/bin/env python3
"""Exact branch-only actual Font/Text fixture. No Mixins/Game/UI/GPU proof."""
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import signal
import subprocess
import sys
import time

ROOT = Path(__file__).resolve().parents[3]
HERE = Path(__file__).resolve().parent
FLOOR = 10 * 1024**3 + 128 * 1024**2

def existing_parent(path):
    path = Path(path).expanduser().resolve()
    while not path.exists():
        if path.parent == path: raise ValueError('No existing output filesystem: ' + str(path))
        path = path.parent
    return path

def write_filesystems(work, env):
    # apt writes /; rustup may use a different mount from workspace and Cargo.
    return [existing_parent(p) for p in [ROOT, work, env['CARGO_HOME'], Path('/'),
            env.get('RUSTUP_HOME', str(Path.home()/'.rustup'))]]

def sha(data): return hashlib.sha256(data).hexdigest()
def read_inputs(): return json.loads((HERE / 'inputs.json').read_text())
def git(root, *args):
    return subprocess.check_output(['git', '-C', str(root), *args], text=True).strip()
def seal(root, tracked=False):
    paths = ([root / p for p in git(root, 'ls-files').splitlines()] if tracked else
             [p for p in root.rglob('*') if p.is_file() and '.git' not in p.relative_to(root).parts])
    return {str(p.relative_to(root)): sha(p.read_bytes()) for p in sorted(paths)}
def assert_seal(root, expected, tracked=False):
    if seal(root, tracked) != expected: raise ValueError('Source/lock/file-set drift: ' + str(root))
def validate_inputs(config, root):
    for path, digest in config['source_sha256'].items():
        if sha((root / path).read_bytes()) != digest: raise ValueError('Frozen source mismatch: ' + path)
    api = 'pluto-lib/mixins/pluto_native_creative/inventory-api/'
    actual = {str(p.relative_to(root)) for p in (root / api).rglob('*') if p.is_file()}
    if actual != {p for p in config['source_sha256'] if p.startswith(api)}:
        raise ValueError('Frozen API complete file set differs')
def validate_tests(log, config):
    results = re.findall(r'^test (consumer_tests::\w+) \.\.\. (\w+)\s*$', log, re.M)
    if len(results) != len(config['test_names']) or {name for name, _ in results} != set(config['test_names']):
        raise ValueError('Missing, duplicate or unexpected native test names')
    if any(result != 'ok' for _, result in results): raise ValueError('Native test not passed')
    summaries = re.findall(r'^test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; (\d+) filtered out;', log, re.M)
    if summaries != [('12', '0', '0', '0', str(config['filtered_out']))]:
        raise ValueError('Native result/count mismatch or skipped fixture')
def native_rustc(log):
    for name in ['horizon_client_core', 'horizon_client_render', 'pluto_guide_text_tests']:
        if not any('Running `' in line and 'rustc' in line and '--crate-name '+name+' ' in line for line in log.splitlines()):
            raise ValueError('Missing actual compilation invocation: ' + name)
def out_dirs(target):
    return {str(p.relative_to(target)): sha(p.read_bytes())
            for out in sorted(target.glob('debug/build/*/out')) for p in sorted(out.rglob('*')) if p.is_file()}

class Runner:
    def __init__(self, work, env):
        self.work, self.env = work, env
        self.minimum_free = None
    def guard(self, reserve=0):
        free = min(shutil.disk_usage(p).free for p in write_filesystems(self.work, self.env))
        self.minimum_free = free if self.minimum_free is None else min(self.minimum_free, free)
        if free < FLOOR + reserve: raise RuntimeError('Disk floor reached; own process group stopped')
    def run(self, name, command, cwd=None, timeout=2400):
        self.guard()
        path = self.work / (name + '.log')
        print('PHASE', name, json.dumps(command), flush=True)
        started = time.monotonic()
        with path.open('w') as log:
            proc = subprocess.Popen(command, cwd=cwd or ROOT, env=self.env, stdout=log, stderr=subprocess.STDOUT, start_new_session=True)
            try:
                while proc.poll() is None:
                    self.guard()
                    if time.monotonic() - started > timeout: raise TimeoutError(name)
                    time.sleep(0.2)
            finally:
                # Never leave a child behind merely because the direct leader exited.
                # TERM has one bounded second; KILL and reap are bounded separately.
                if proc.poll() is None:
                    try: os.killpg(proc.pid, signal.SIGTERM)
                    except ProcessLookupError: pass
                    deadline = time.monotonic() + 1.0
                    while proc.poll() is None and time.monotonic() < deadline:
                        time.sleep(0.05)
                try: os.killpg(proc.pid, signal.SIGKILL)
                except ProcessLookupError: pass
                proc.wait(timeout=2)
                print(path.read_text(errors='replace'), end='', flush=True)
        self.guard()
        print('TERMINAL', name, proc.returncode, 'seconds', round(time.monotonic()-started, 3), flush=True)
        if proc.returncode != 0: raise RuntimeError(name + ' exited ' + str(proc.returncode))
        return path.read_text()

def main():
    config = read_inputs()
    if (os.environ.get('GITHUB_REPOSITORY') != config['repository'] or
        os.environ.get('GITHUB_EVENT_NAME') != 'push' or
        os.environ.get('GITHUB_REF') != 'refs/heads/' + config['branch'] or
        git(ROOT, 'rev-parse', 'HEAD') != os.environ.get('GITHUB_SHA') or
        git(ROOT, 'status', '--porcelain', '--untracked-files=all')):
        raise ValueError('Require exact clean authorized CI checkout/event')
    event = json.loads(Path(os.environ['GITHUB_EVENT_PATH']).read_text())
    if event.get('repository', {}).get('private') is not False:
        raise ValueError('Require public-repository standard-runner scope')
    validate_inputs(config, ROOT)
    original = seal(ROOT, True)
    work = Path(os.environ['RUNNER_TEMP']) / ('guide-font-' + os.environ['GITHUB_SHA'])
    if min(shutil.disk_usage(p).free for p in write_filesystems(work, {**os.environ, 'CARGO_HOME':str(work/'cargo-home')})) < FLOOR:
        raise RuntimeError('Preflight disk floor before any setup/output')
    work.mkdir()  # Fresh runner-owned path only; never remove arbitrary/stale output.
    target, host, fixture = work/'target', work/'horizon', work/'fixture'
    env = os.environ.copy()
    for key in ['RUSTFLAGS', 'CARGO_ENCODED_RUSTFLAGS', 'RUSTC_WRAPPER', 'RUSTC_WORKSPACE_WRAPPER']:
        env.pop(key, None)
    env.update(RUSTUP_TOOLCHAIN='1.95.0', CARGO_HOME=str(work/'cargo-home'),
               CARGO_TARGET_DIR=str(target), CARGO_BUILD_JOBS='1',
               CARGO_PROFILE_DEV_DEBUG='0', CARGO_PROFILE_TEST_DEBUG='0', CARGO_INCREMENTAL='0',
               TMPDIR=str(work/'tmp'), PYTHONDONTWRITEBYTECODE='1', CARGO_TERM_COLOR='never')
    Path(env['CARGO_HOME']).mkdir(); Path(env['TMPDIR']).mkdir()
    runner = Runner(work, env)
    official = before_fixture = generated = None
    success = False
    try:
        # Fail closed before fixed setup if its cold installation lacks headroom.
        runner.guard(reserve=2 * 1024**3)
        runner.run('apt-update', ['sudo','apt-get','update'], timeout=600)
        runner.run('apt-install', ['sudo','apt-get','install','-y','--no-install-recommends','libasound2-dev','libudev-dev'], timeout=600)
        runner.run('toolchain-install', ['rustup','toolchain','install','1.95.0','--profile','minimal'], timeout=600)
        runner.run('source-guards', ['python3', str(ROOT/'tconstruct/tests/guide-text-migration/test_source.py')])
        runner.run('official-init', ['git', 'init', str(host)])
        runner.run('official-remote', ['git', '-C', str(host), 'remote', 'add', 'origin', config['official_url']])
        runner.run('official-fetch', ['git', '-C', str(host), 'fetch', '--depth', '1', 'origin', config['official_revision']])
        runner.run('official-checkout', ['git', '-C', str(host), 'checkout', '--detach', 'FETCH_HEAD'])
        if git(host,'rev-parse','HEAD') != config['official_revision'] or git(host,'remote','get-url','origin') != config['official_url']:
            raise ValueError('Official source identity mismatch')
        if sha((host/'Cargo.lock').read_bytes()) != config['official_lock_sha256']: raise ValueError('Official lock mismatch')
        official = seal(host)
        # Preparation's metadata is offline. Fetch exact official locked packages first on a cold runner.
        runner.run('official-locked-fetch', ['cargo', 'fetch', '--locked', '--manifest-path', str(host/'Cargo.toml')])
        assert_seal(host, official)
        preparer = ROOT/'tconstruct/tests/guide-text-migration/prepare.py'
        runner.run('prepare', ['python3', str(preparer), '--horizon-source', str(host), '--output', str(fixture)])
        before_fixture = seal(fixture)
        command = ['cargo','test','--locked','--offline','--manifest-path',str(fixture/'Cargo.toml'),'--lib']
        built = runner.run('compile', command + ['--no-run','-vv'])
        native_rustc(built)
        generated = out_dirs(target)
        runner.run('verify-after-compile', ['python3', str(preparer), '--output', str(fixture), '--verify'])
        assert_seal(fixture, before_fixture); assert_seal(host, official); assert_seal(ROOT, original, True)
        result = runner.run('font12', command + ['consumer_tests::','--','--test-threads=1'])
        validate_tests(result, config)
        runner.run('verify-after-tests', ['python3', str(preparer), '--output', str(fixture), '--verify'])
        success = True
    finally:
        # Final exact sources, lock, prepared inputs and used generated inputs must survive both Cargo runs.
        assert_seal(ROOT, original, True)
        if official is not None: assert_seal(host, official)
        if before_fixture is not None: assert_seal(fixture, before_fixture)
        if generated is not None and out_dirs(target) != generated: raise ValueError('Generated native OUT_DIR inputs changed during tests')
        report = {'passed':success,'scope':'actual Font/Text synthetic CPU atlas only; no Game, Mixins apply, GPU, UI or installed consumer proof',
                  'head':os.environ['GITHUB_SHA'],'official_revision':config['official_revision'],
                  'minimum_free_bytes':runner.minimum_free,'floor_bytes':FLOOR,
                  'source_seal_sha256':sha(json.dumps(original,sort_keys=True).encode()),
                  'official_seal_sha256':sha(json.dumps(official,sort_keys=True).encode()) if official else None}
        print('FINAL_RECEIPT',json.dumps(report,sort_keys=True),flush=True)
        with open(os.environ['GITHUB_STEP_SUMMARY'],'a') as summary:
            summary.write('## Guide Font12 bounded proof\n\n```json\n'+json.dumps(report,indent=2)+'\n```\n')

if __name__ == '__main__': main()
