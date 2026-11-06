#!/usr/bin/env python3
"""Read-only negative capability probe on the selected official host.

No apply, transformation, compilation/test, host edits or fallback.
The official planner invokes Cargo metadata; source the approved environment
first so Cargo is available on PATH. The single
state must be rejected for the public-layout contract, not an unrelated error.
Use only in the coordinator's serial slot. An expected rejection proves no
runtime capability and must never be treated as a passing gameplay test.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import signal
import subprocess
import time
from prepare_render_fixture import clean_official

HORIZON = '624c4f6265c02296fbe8a80f029dd0d987fb94c0'
MIXINS = 'e8cd0aa38cffef49cb2e088ef716baa107148c02'
CLI_SHA256 = '50fd3105daafd09525889c11c2374a727dbd41b0bad0a5b8714c01b0d06866ff'
EXPECTED = 'state requires a private field or non_exhaustive struct; external construction would break'
FLOOR = 10 * 1024**3 + 128 * 1024**2
ROOT = Path(__file__).resolve().parents[1]


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def official_export(host, export):
    """Check every tracked official byte, with symlink docs materialized safely."""
    clean_official(host, HORIZON)
    entries = subprocess.check_output(['git', '-C', str(host), 'ls-tree', '-rz', 'HEAD'])
    expected = {'.horizon-revision'}
    hashes = {}
    for entry in entries.split(b'\0'):
        if not entry:
            continue
        header, path = entry.split(b'\t', 1)
        mode, kind, oid = header.decode().split()
        name = path.decode()
        if kind != 'blob' or mode not in ('100644', '100755', '120000'):
            raise ValueError('Unsupported official source entry: ' + name)
        actual = export / name
        if actual.is_symlink() or not actual.is_file():
            raise ValueError('Export requires regular source bytes: ' + name)
        data = actual.read_bytes()
        if mode == '120000':
            # The prepared official export materializes CLAUDE.md -> AGENTS.md.
            # This does not change Rust/Cargo source and does not bypass a read.
            target = (host / name).resolve()
            if not target.is_relative_to(host) or data != target.read_bytes():
                raise ValueError('Official link materialization changed: ' + name)
        elif hashlib.sha1(b'blob ' + str(len(data)).encode() + b'\0' + data).hexdigest() != oid:
            raise ValueError('Export differs from exact official blob: ' + name)
        hashes[name] = hashlib.sha256(data).hexdigest()
        expected.add(name)
    actual = {str(p.relative_to(export)) for p in export.rglob('*') if p.is_file()}
    if actual != expected or (export / '.horizon-revision').read_text().strip() != HORIZON:
        raise ValueError('Unexpected official export contents or target')
    return hashes


def rejected(returncode, stderr, plan_exists):
    if returncode != 1 or plan_exists:
        return False
    try:
        diagnostic = json.loads(stderr)
    except (ValueError, TypeError):
        return False
    return (isinstance(diagnostic, dict) and diagnostic.get('code') == 'E_STATE'
            and diagnostic.get('message') == EXPECTED)


def run(host, export, mixins, cli, output):
    host, export, mixins, cli, output = [p.resolve() for p in (host, export, mixins, cli, output)]
    def git(*args):
        return subprocess.check_output(['git', '-C', str(mixins), *args], text=True).strip()
    if (git('remote', 'get-url', 'origin') != 'https://github.com/horizon-rust/horizon-mixins.git'
            or git('rev-parse', 'HEAD') != MIXINS or git('status', '--porcelain', '--untracked-files=all')):
        raise ValueError('Require exact clean official Mixins source')
    if sha(cli) != CLI_SHA256:
        raise ValueError('Require previously built and recorded official CLI bytes')
    inputs = official_export(host, export)
    manifest = ROOT / 'tests/probes/effect-instance-state/horizon-mixin.toml'
    if output.exists():
        raise ValueError('Output must be new; preserve previous evidence')
    output.mkdir(parents=True)
    plan = output / 'unexpected-plan.json'
    command = [str(cli), 'plan', '--workspace', str(export), '--mods', str(manifest.parent), '--out', str(plan)]
    record = {'horizon_revision': HORIZON, 'mixins_revision': MIXINS, 'command': command,
              'cli_sha256': sha(cli), 'manifest_sha256': sha(manifest),
              'official_files_verified': len(inputs), 'guard_free_bytes': FLOOR,
              'minimum_free_bytes': shutil.disk_usage(output).free,
              'limits': ['Negative planner result only', 'No apply/build/gameplay',
                         'Constructor coverage and field types are not reached by this rejection']}
    if record['minimum_free_bytes'] <= FLOOR:
        raise ValueError('Disk floor before probe')
    start = time.monotonic()
    stopped = False
    with (output/'stdout.log').open('w') as out, (output/'stderr.log').open('w') as err:
        proc = subprocess.Popen(command, stdout=out, stderr=err, start_new_session=True)
        try:
            while proc.poll() is None:
                free = shutil.disk_usage(output).free
                record['minimum_free_bytes'] = min(record['minimum_free_bytes'], free)
                if free <= FLOOR:
                    stopped = True
                    break
                time.sleep(0.25)
        finally:
            if proc.poll() is None:
                os.killpg(proc.pid, signal.SIGINT)
                try:
                    proc.wait(timeout=10)
                except subprocess.TimeoutExpired:
                    os.killpg(proc.pid, signal.SIGTERM)
                    proc.wait()
    unchanged = inputs == official_export(host, export)
    record.update(returncode=proc.returncode, disk_guard_stopped=stopped,
                  elapsed_seconds=time.monotonic()-start, official_inputs_unchanged=unchanged,
                  stdout_sha256=sha(output/'stdout.log'), stderr_sha256=sha(output/'stderr.log'))
    record['expected_rejection_observed'] = (not stopped and unchanged and
        rejected(proc.returncode, (output/'stderr.log').read_text(), plan.exists()))
    (output/'result.json').write_text(json.dumps(record, indent=2)+'\n')
    print(json.dumps(record, indent=2))
    if not record['expected_rejection_observed']:
        raise SystemExit(1)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    for arg in ('horizon-source', 'official-export', 'mixins-source', 'cli', 'output'):
        parser.add_argument('--'+arg, required=True, type=Path)
    args = parser.parse_args()
    run(args.horizon_source, args.official_export, args.mixins_source, args.cli, args.output)
