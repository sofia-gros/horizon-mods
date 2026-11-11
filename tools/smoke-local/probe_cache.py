"""Content-checked probe caches. Guards still run on every invocation."""
from contextlib import contextmanager
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys

IGNORED = {'.git', 'target', '__pycache__'}


def file_digest(path):
    with Path(path).open('rb') as source:
        return hashlib.file_digest(source, 'sha256').hexdigest()


def tree_digest(root):
    root = Path(root)
    digest = hashlib.sha256()
    for directory, dirs, files in os.walk(root, followlinks=False):
        dirs[:] = sorted(name for name in dirs if name not in IGNORED)
        for name in dirs + files:
            if (Path(directory) / name).is_symlink():
                raise ValueError('probe cache inputs must not contain symbolic links')
        for name in sorted(files):
            path = Path(directory) / name
            digest.update(path.relative_to(root).as_posix().encode())
            digest.update(b'\0')
            digest.update(bytes.fromhex(file_digest(path)))
    return digest.hexdigest()


def sync_tree(source, destination):
    """Keep stable paths and untouched files' mtimes; remove stale inputs only."""
    source, destination = Path(source), Path(destination)
    destination.mkdir(parents=True, exist_ok=True)
    for old in destination.iterdir():
        if old.name in IGNORED or not (source / old.name).exists():
            if old.is_dir() and not old.is_symlink():
                shutil.rmtree(old)
            else:
                old.unlink()
    for item in source.iterdir():
        if item.name in IGNORED:
            continue
        if item.is_symlink():
            raise ValueError('probe bundle must not contain symbolic links')
        target = destination / item.name
        if target.is_symlink():
            raise ValueError('probe cache destination must not contain symbolic links')
        if item.is_dir():
            if target.exists() and not target.is_dir():
                target.unlink()
            sync_tree(item, target)
        else:
            if target.is_dir():
                shutil.rmtree(target)
            if not target.is_file() or file_digest(item) != file_digest(target):
                shutil.copy2(item, target)


@contextmanager
def fresh_output(output):
    """The launcher requires a new output path; roll back failed replacements."""
    output = Path(output)
    backup = output.with_name(output.name + '.previous')
    if backup.exists() or backup.is_symlink() or output.is_symlink():
        raise ValueError('inspect existing output backup/symlink before rebuilding')
    if output.exists():
        output.rename(backup)
    try:
        yield
    except BaseException:
        if output.exists():
            shutil.rmtree(output)
        if backup.exists():
            backup.rename(output)
        raise
    else:
        if backup.exists():
            shutil.rmtree(backup)


def read_receipt(path):
    try:
        value = json.loads(Path(path).read_text(encoding='utf-8'))
        return value if isinstance(value, dict) else {}
    except (OSError, ValueError):
        return {}


def write_receipt(path, value):
    path = Path(path)
    temporary = path.with_name(path.name + '.tmp')
    temporary.write_text(json.dumps(value, sort_keys=True) + '\n', encoding='utf-8', newline='\n')
    temporary.replace(path)


def guarded_plan(plan, receipt, key, produce, guard):
    """Never reuse a failed/tampered plan, and never cache a guard decision."""
    plan = Path(plan)
    old = read_receipt(receipt)
    hit = (old.get('schema') == 1 and old.get('key') == key and plan.is_file()
           and old.get('plan') == file_digest(plan))
    Path(receipt).unlink(missing_ok=True)
    if not hit:
        produce()
    guard()
    if plan.is_file():
        write_receipt(receipt, {'schema': 1, 'key': key, 'plan': file_digest(plan)})
    return hit


def artifact_digests(output):
    output = Path(output)
    result = {}
    for name in ('horizon-engine', 'horizon-ctl', 'horizon-server', 'horizon-prepare'):
        path = next((p for p in (output / (name+'.exe'), output / name) if p.is_file()), None)
        if path:
            result[path.name] = file_digest(path)
    report = output / 'native-mods.json'
    if not report.is_file():
        return {}
    result[report.name] = file_digest(report)
    if not any(n.startswith('horizon-engine') for n in result) or not any(n.startswith('horizon-ctl') for n in result):
        return {}
    # Dynamic runtime libraries, when supplied, are also part of the artifact.
    for pattern in ('*.dll', '*.so', '*.dylib'):
        for path in output.rglob(pattern):
            result[path.relative_to(output).as_posix()] = file_digest(path)
    return result


def build_key(bundles, launcher, preflight, full, output):
    try:
        compiler = subprocess.run(['rustc', '-vV'], check=True, capture_output=True, text=True).stdout
    except (OSError, subprocess.CalledProcessError):
        return None  # launcher may provision Rust itself; do not guess its inputs
    variables = {'RUSTFLAGS', 'CARGO_ENCODED_RUSTFLAGS', 'RUSTUP_TOOLCHAIN', 'RUSTC',
                 'RUSTC_WRAPPER', 'RUSTC_WORKSPACE_WRAPPER', 'CARGO_BUILD_TARGET',
                 'CARGO_HOME', 'CARGO_TARGET_DIR', 'CC', 'CXX', 'CFLAGS', 'CXXFLAGS',
                 'SOURCE_DATE_EPOCH', 'RUSTC_BOOTSTRAP', 'HORIZON_LAUNCHER_HOME'}
    variables.update(k for k in os.environ if k.startswith('CARGO_PROFILE_'))
    configs = {}
    cargo_home = Path(os.environ.get('CARGO_HOME', Path.home() / '.cargo'))
    config_dirs = {cargo_home}
    for start in (Path.cwd().resolve(), Path(bundles).resolve(), Path(output).resolve()):
        config_dirs.update(parent / '.cargo' for parent in (start, *start.parents))
    for directory in sorted(config_dirs):
        for name in ('config', 'config.toml'):
            path = directory / name
            if path.is_file():
                configs[str(path.resolve())] = file_digest(path)
    return {'bundles': tree_digest(bundles), 'launcher': file_digest(launcher),
            'preflight': preflight, 'compiler': compiler, 'platform': sys.platform,
            'full': bool(full), 'profile': 'dev', 'output': str(Path(output).resolve()),
            'env': {k: os.environ.get(k) for k in sorted(variables)}, 'cargo_config': configs}


def reusable_build(receipt, key, output):
    if key is None:
        return False
    old = read_receipt(receipt)
    if old.get('schema') != 1 or old.get('key') != key:
        return False
    actual = artifact_digests(output)
    return bool(actual) and actual == old.get('artifacts')


def record_build(receipt, key, output):
    if key is None:
        return
    actual = artifact_digests(output)
    if not actual:
        raise ValueError('cannot record an incomplete native build')
    write_receipt(receipt, {'schema': 1, 'key': key, 'artifacts': actual})
