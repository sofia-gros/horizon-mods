"""Read-only admission of the official Rust 1.99.1 Windows GNU formatter.

No download, extraction, installation, PATH discovery or compiler execution.
Existing guard comparisons and regeneration hunks remain unchanged.
"""
import hashlib
import io
from pathlib import Path
import re
import tarfile

from playtest_pluto_inputs import directory, plain, signature

URL = 'https://static.rust-lang.org/dist/2026-10-15/rustfmt-1.99.1-x86_64-pc-windows-gnu.tar.gz'
ARCHIVE_SHA256 = '488f4cdcec501721fb2986a4c36c8180d5606f5b569f942fb7c8886f36a7e2aa'
VERSION = '1.10.0'
COMMIT = '58cf8f9c35b355199727627d07ae34a4e2846d67'
MEMBER = 'rustfmt-1.99.1-x86_64-pc-windows-gnu/rustfmt-preview/bin/rustfmt.exe'
ARCHIVE_LIMIT = 64 * 1024**2
EXPANDED_LIMIT = 256 * 1024**2
MEMBER_LIMIT = 512
GUARD_ROUTING = {
    'check_terrain_cell_plan.py': 'b8b551fb45d6a5b118fa6d14c964a7e908c61f64a70749d18457b92ec151df60',
    'check_damage_equipment_plan.py': '9843dbe27c62bb1e3765f6cc70c2d1079c21cbccb092f8763dc05b32f72129e3',
}


def sha(raw): return hashlib.sha256(raw).hexdigest()


def bounded_file(path, limit):
    path = Path(path); directory(path.parent)
    before = path.lstat(); plain(before)
    if not path.is_file() or not 0 < before.st_size <= limit:
        raise ValueError('Formatter input is missing, empty or exceeds its read budget')
    with path.open('rb') as stream:
        import os
        opened = os.fstat(stream.fileno()); plain(opened)
        if signature(opened) != signature(before): raise ValueError('Formatter input changed before read')
        raw = stream.read(limit + 1)
        after = os.fstat(stream.fileno()); plain(after)
    final = path.lstat(); plain(final)
    if len(raw) != before.st_size or signature(after) != signature(before) or signature(final) != signature(before):
        raise ValueError('Formatter input changed during read')
    return raw


def archive_binary(raw):
    if not 0 < len(raw) <= ARCHIVE_LIMIT or sha(raw) != ARCHIVE_SHA256:
        raise ValueError('Expected the exact official Rust 1.99.1 GNU rustfmt archive: ' + URL)
    seen = set(); total = 0; binary = None
    with tarfile.open(fileobj=io.BytesIO(raw), mode='r:gz') as archive:
        for member in archive:
            name = member.name
            if (name.startswith('/') or '\\' in name or ':' in name or
                    any(p in ('', '.', '..') for p in name.rstrip('/').split('/')) or name in seen):
                raise ValueError('Unsafe or duplicate formatter archive member')
            seen.add(name)
            if len(seen) > MEMBER_LIMIT or not (member.isfile() or member.isdir()):
                raise ValueError('Unsupported formatter archive member/count')
            if member.size < 0: raise ValueError('Negative formatter archive member size')
            total += member.size
            if total > EXPANDED_LIMIT: raise ValueError('Formatter archive expansion budget exceeded')
            if name == MEMBER:
                if not member.isfile() or not 0 < member.size <= ARCHIVE_LIMIT:
                    raise ValueError('Formatter archive executable is not a bounded regular file')
                with archive.extractfile(member) as stream:
                    binary = stream.read(ARCHIVE_LIMIT + 1)
                if len(binary) != member.size: raise ValueError('Truncated formatter archive executable')
    if binary is None: raise ValueError('Official formatter executable member is missing')
    return binary


def admit(toolchain, archive_path):
    toolchain = directory(toolchain)
    archive_path = Path(archive_path)
    raw = bounded_file(archive_path, ARCHIVE_LIMIT)
    expected = archive_binary(raw)
    executable = toolchain / 'bin/rustfmt.exe'
    actual = bounded_file(executable, ARCHIVE_LIMIT)
    if actual != expected: raise ValueError('Selected rustfmt.exe differs from the pinned official archive')
    return {'archive_url': URL, 'archive_sha256': sha(raw), 'archive_bytes': len(raw),
            'executable_sha256': sha(actual), 'executable_bytes': len(actual),
            'package_version': VERSION, 'package_commit': COMMIT}


def require_guard_routing(pluto):
    for name, expected in GUARD_ROUTING.items():
        if sha(bounded_file(Path(pluto)/'tools'/name, 64*1024)) != expected:
            raise ValueError('Guard does not match reviewed explicit formatter routing: '+name)


def validate_version(text):
    # The distribution manifest pins version and commit. The build date is
    # displayed but is not guessed from the later distribution publication date.
    match = re.fullmatch(r'rustfmt ([0-9]+\.[0-9]+\.[0-9]+)-stable \(([0-9a-f]{7,40}) ([0-9]{4}-[0-9]{2}-[0-9]{2})\)', text)
    if not match or match[1] != VERSION or not COMMIT.startswith(match[2]):
        raise ValueError('Formatter version/commit differs from official Rust 1.99.1 rustfmt')


