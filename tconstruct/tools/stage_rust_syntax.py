"""Non-writing Rust syntax gate for the one repaired generated source file."""
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import time

import stage_float_range_repair as repair

MAX_INPUT = 4 * 1024**2
MAX_STDERR = 1024**2
TIMEOUT = 30
RECEIPT = 'rust-syntax-receipt.json'


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def selected_formatter():
    value = os.environ.get('PLUTO_GUARD_RUSTFMT', 'rustfmt')
    if 'PLUTO_GUARD_RUSTFMT' in os.environ and not Path(value).is_absolute():
        raise ValueError('Selected formatter must be absolute')
    resolved = value if Path(value).is_absolute() else shutil.which(value)
    if not resolved:
        raise ValueError('The existing selected rustfmt is required for generated Rust syntax')
    return Path(resolved).resolve(strict=True)


def parse_file(target, receipt_path, formatter):
    target = Path(target); receipt_path = Path(receipt_path); formatter = Path(formatter)
    if receipt_path.exists() or receipt_path.is_symlink():
        raise ValueError('Rust syntax receipt must be fresh')
    raw = repair.read(target, MAX_INPUT)
    binary = repair.read(formatter, 64 * 1024**2)
    result = {'schema': 1, 'kind': 'rustfmt-stdin-syntax', 'passed': False,
              'syntax_only': True, 'type_checked': False, 'target': repair.TARGET,
              'input_bytes': len(raw), 'input_sha256': sha(raw),
              'formatter': str(formatter), 'formatter_sha256': sha(binary),
              'edition': '2024', 'stdin_nonrecursive': True, 'emit': 'stdout',
              'writes_source': False, 'timeout_seconds': TIMEOUT}
    error = None; p = None; start = time.monotonic()
    try:
        with tempfile.TemporaryDirectory(prefix='rust-syntax-', dir=receipt_path.parent) as temp:
            temp = Path(temp); config = temp/'rustfmt.toml'; config.write_bytes(b'')
            copied_input = temp/'stdin'; copied_input.write_bytes(raw)
            stderr_path = temp/'stderr'
            try:
                with copied_input.open('rb') as stdin, stderr_path.open('xb') as stderr:
                    # Text input disables out-of-line module traversal in the fixed
                    # Rustfmt implementation. An empty explicit config avoids parents.
                    argv = [str(formatter), '--edition', '2024', '--emit', 'stdout', '--config-path', str(config)]
                    p = subprocess.Popen(argv, stdin=stdin, stdout=subprocess.DEVNULL, stderr=stderr, cwd=temp)
                    while p.poll() is None:
                        if time.monotonic()-start > TIMEOUT:
                            raise TimeoutError('Rust syntax parser time limit')
                        if stderr_path.stat().st_size > MAX_STDERR:
                            raise ValueError('Rust syntax diagnostic budget')
                        time.sleep(.01)
                    result['returncode'] = p.wait()
            finally:
                if p is not None and p.poll() is None:
                    p.kill(); p.wait(timeout=5)
            if stderr_path.stat().st_size > MAX_STDERR:
                raise ValueError('Rust syntax diagnostic budget')
            diagnostic = repair.read(stderr_path, MAX_STDERR)
            result.update(stderr_bytes=len(diagnostic), stderr_sha256=sha(diagnostic),
                          diagnostic=diagnostic.decode('utf-8', errors='replace')[:4096])
            if result['returncode'] != 0 or diagnostic:
                raise ValueError('Generated Rust syntax was not accepted')
            result['source_unchanged'] = repair.read(target, MAX_INPUT) == raw
            result['formatter_unchanged'] = repair.read(formatter, 64*1024**2) == binary
            if not result['source_unchanged'] or not result['formatter_unchanged']:
                raise ValueError('Rust syntax input or formatter changed')
            result['passed'] = True
    except Exception as e:
        error = e; result['error'] = type(e).__name__ + ': ' + str(e)
    finally:
        if p is not None and p.poll() is None:
            p.kill(); p.wait(timeout=5)
        result['seconds'] = time.monotonic()-start
        payload = json.dumps(result, sort_keys=True, indent=2).encode()+b'\n'
        if len(payload) > 16384:
            raise ValueError('Rust syntax receipt budget')
        with receipt_path.open('xb') as stream:
            stream.write(payload); stream.flush(); os.fsync(stream.fileno())
        if repair.read(receipt_path, 16384) != payload:
            raise ValueError('Rust syntax receipt readback changed')
    if error is not None:
        raise error
    return result


def check_generated(modded, output):
    return parse_file(Path(modded)/repair.TARGET, Path(output)/RECEIPT, selected_formatter())
