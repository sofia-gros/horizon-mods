#!/usr/bin/env python3
"""Fingerprint Pluto inputs for the local Windows playtest cache, not compatibility.

Read only; no compile, checkout, copying, deletion or guard bypass. Call against
quiescent inputs. Stat/recheck rejects ordinary drift, not an adversarial TOCTOU
filesystem. Reparse points, symlinks and special files are never traversed.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import stat

MAX_FILES = 10000
MAX_FILE_BYTES = 64 * 1024 * 1024
MAX_TOTAL_BYTES = 256 * 1024 * 1024
REPARSE = getattr(stat, 'FILE_ATTRIBUTE_REPARSE_POINT', 0x400)


def plain(info):
    if stat.S_ISLNK(info.st_mode) or getattr(info, 'st_file_attributes', 0) & REPARSE:
        raise ValueError('Pluto cache input is a link or reparse point')
    if not (stat.S_ISDIR(info.st_mode) or stat.S_ISREG(info.st_mode)):
        raise ValueError('Pluto cache input is not a regular file/directory')


def directory(path):
    path = Path(os.path.abspath(path))
    for part in (*reversed(path.parents), path):
        info = part.lstat(); plain(info)
        if not stat.S_ISDIR(info.st_mode):
            raise ValueError('Pluto cache root is not a directory')
    return path


def signature(info):
    return (info.st_dev, info.st_ino, info.st_mode, info.st_size, info.st_mtime_ns)


def file_hash(path, before):
    if before.st_size > MAX_FILE_BYTES:
        raise ValueError('Pluto cache file exceeds size limit')
    total = 0; digest = hashlib.sha256()
    with path.open('rb') as stream:
        opened = os.fstat(stream.fileno()); plain(opened)
        if signature(opened) != signature(before):
            raise ValueError('Pluto cache input changed before read')
        while block := stream.read(65536):
            total += len(block)
            if total > MAX_FILE_BYTES:
                raise ValueError('Pluto cache file grew past size limit')
            digest.update(block)
        if signature(os.fstat(stream.fileno())) != signature(before):
            raise ValueError('Pluto cache input changed during read')
    after = path.lstat(); plain(after)
    if signature(after) != signature(before) or total != before.st_size:
        raise ValueError('Pluto cache input changed after read')
    return digest.hexdigest(), total


def fingerprint(bundle, pluto_root):
    bundle = directory(bundle); pluto_root = directory(pluto_root)
    roots = [('selected_bundle', bundle), ('reviewed_native', pluto_root/'mixins/pluto_native_creative'),
             ('guard_tools', pluto_root/'tools'), ('content_helpers', pluto_root/'pluto_lib')]
    if not (bundle/'horizon-mixin.toml').is_file():
        raise ValueError('Selected Pluto bundle has no manifest')
    rows = []; total_bytes = 0
    def walk(root, current, label):
        nonlocal total_bytes
        for path in sorted(current.iterdir(), key=lambda p: p.name):
            info = path.lstat(); plain(info)
            # Interpreter cache is not code/build input and can appear on imports.
            if path.name == '__pycache__' and stat.S_ISDIR(info.st_mode):
                continue
            if path.suffix == '.pyc' and stat.S_ISREG(info.st_mode):
                continue
            relative = path.relative_to(root).as_posix()
            if stat.S_ISDIR(info.st_mode):
                walk(root, path, label)
            else:
                digest, size = file_hash(path, info); total_bytes += size
                if len(rows) >= MAX_FILES or total_bytes > MAX_TOTAL_BYTES:
                    raise ValueError('Pluto cache input budget exceeded')
                rows.append([label, relative, digest, size])
    for label, root in roots:
        root = directory(root); walk(root, root, label)
    payload = {'schema': 1, 'files': rows}
    return hashlib.sha256(json.dumps(payload, sort_keys=True, separators=(',', ':')).encode()).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--pluto-bundle', type=Path, required=True)
    parser.add_argument('--pluto-root', type=Path, required=True)
    args = parser.parse_args()
    try:
        print(fingerprint(args.pluto_bundle, args.pluto_root))
    except (OSError, ValueError) as exc:
        parser.exit(1, 'Cannot fingerprint Pluto playtest inputs: '+str(exc)+'\n')


if __name__ == '__main__':
    main()
