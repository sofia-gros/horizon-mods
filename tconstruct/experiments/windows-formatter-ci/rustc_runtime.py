"""One pinned official Rust component, streamed into a fresh local prefix.

Only the formatter controller may call this preparer after its own Job admission.
No installer, subprocess, alternate source, compiler or PATH discovery is used.
Budgets are rejection limits, not measured archive size or build-growth estimates.
"""
from contextlib import contextmanager
import ctypes as C
import hashlib
import gzip
import json
import os
from pathlib import Path, PurePosixPath
import re
import shutil
import stat
import tarfile
import time
import urllib.request
import sys

sys.path.insert(0, str(Path(__file__).resolve().parents[2]/'tools'))

from playtest_pluto_inputs import directory, plain, read_signature, require_opened_file

URL = 'https://static.rust-lang.org/dist/2026-10-15/rustc-1.99.1-x86_64-pc-windows-gnu.tar.gz'
SHA256 = '43b17e9a9134ea72562bfbb23728a830f671ea9be1db6a72597a8d240e986b35'
TOP = 'rustc-1.99.1-x86_64-pc-windows-gnu'
COMMIT = '58cf8f9c35b355199727627d07ae34a4e2846d67'
ARCHIVE_LIMIT = 512 * 1024**2
EXPANDED_LIMIT = 2 * 1024**3
FILE_LIMIT = 1024**3
MEMBER_LIMIT = 20000
RESERVE = 4 * 1024**3
RAM_FLOOR = 512 * 1024**2
REQUIRED_DLLS = ('rustc_driver-737f10a507f50706.dll', 'std-f903e3d16df36da5.dll',
                 'libgcc_s_seh-1.dll', 'libwinpthread-1.dll')


def sha(raw): return hashlib.sha256(raw).hexdigest()


@contextmanager
def opened(path, limit):
    path = Path(path); directory(path.parent)
    before = path.lstat(); plain(before)
    if not stat.S_ISREG(before.st_mode) or not 0 <= before.st_size <= limit:
        raise ValueError('Runtime file size/type exceeds budget')
    with path.open('rb') as stream:
        first = os.fstat(stream.fileno()); require_opened_file(path, before, first)
        yield stream, before.st_size
        last = os.fstat(stream.fileno()); plain(last)
        if read_signature(first) != read_signature(last): raise ValueError('Runtime handle changed')
    final = path.lstat(); plain(final)
    if read_signature(before) != read_signature(final): raise ValueError('Runtime path changed')


def file_record(path, limit=FILE_LIMIT):
    with opened(path, limit) as (stream, size):
        h = hashlib.sha256(); count = 0
        while chunk := stream.read(min(65536, limit-count+1)):
            count += len(chunk)
            if count > limit: raise ValueError('Runtime file grew beyond budget')
            h.update(chunk)
        if count != size: raise ValueError('Runtime file length changed')
    return {'bytes': count, 'sha256': h.hexdigest()}


def download(path):
    class NoRedirect(urllib.request.HTTPRedirectHandler):
        def redirect_request(self, *args, **kwargs): raise ValueError('Runtime redirect rejected')
    start = time.monotonic(); count = 0; h = hashlib.sha256()
    with urllib.request.build_opener(NoRedirect()).open(URL, timeout=15) as response, Path(path).open('xb') as out:
        if response.status != 200 or response.geturl() != URL: raise ValueError('Runtime origin changed')
        length = response.headers.get('Content-Length')
        if length is not None and not 0 < int(length) <= ARCHIVE_LIMIT: raise ValueError('Runtime download size')
        while chunk := response.read(min(65536, ARCHIVE_LIMIT-count+1)):
            count += len(chunk)
            if count > ARCHIVE_LIMIT or time.monotonic()-start > 110: raise ValueError('Runtime download budget')
            h.update(chunk); out.write(chunk)
        out.flush(); os.fsync(out.fileno())
    if not count or h.hexdigest() != SHA256: raise ValueError('Official rustc archive differs')
    result = file_record(path, ARCHIVE_LIMIT)
    if result != {'bytes': count, 'sha256': SHA256}: raise ValueError('Runtime download readback differs')
    return result


def safe_name(name):
    parts = name.rstrip('/').split('/')
    if (len(name)>512 or len(parts)>32 or not parts or any(p in ('', '.', '..') or p.endswith((' ', '.')) for p in parts)
            or '\\' in name or ':' in name or any(ord(c) < 32 for c in name)
            or any(re.fullmatch(r'(CON|PRN|AUX|NUL|COM[1-9]|LPT[1-9])(?:\..*)?', p, re.I) for p in parts)):
        raise ValueError('Unsafe runtime archive path')
    return parts


class ExpandedReader:
    def __init__(self, stream): self.stream=stream; self.count=0
    def read(self, size):
        # Include padding/extended headers, not just payload sizes.
        limit=EXPANDED_LIMIT+MEMBER_LIMIT*4096
        if size<0: raise ValueError('Unbounded expanded read')
        data=self.stream.read(min(size, limit-self.count+1)); self.count+=len(data)
        if self.count>limit: raise ValueError('Runtime decompressed stream budget')
        return data


def unpack(path, destination):
    # Verify before decompression; no installer script is executed.
    archive_record = file_record(path, ARCHIVE_LIMIT)
    if archive_record['sha256'] != SHA256: raise ValueError('Official rustc archive differs')
    destination = Path(destination); directory(destination.parent)
    if destination.exists() or destination.is_symlink(): raise ValueError('Runtime prefix is not fresh')
    destination.mkdir()
    seen = set(); total = 0; files = {}; commit = None
    with opened(path, ARCHIVE_LIMIT) as (stream, _), gzip.GzipFile(fileobj=stream) as decompressed, tarfile.open(fileobj=ExpandedReader(decompressed), mode='r|') as archive:
        for member in archive:
            parts = safe_name(member.name); key = '/'.join(parts).casefold()
            if parts[0] != TOP or key in seen: raise ValueError('Runtime root/duplicate path differs')
            seen.add(key)
            if len(seen) > MEMBER_LIMIT or not (member.isdir() or member.isfile()) or member.issparse():
                raise ValueError('Unsupported runtime archive member')
            if not 0 <= member.size <= FILE_LIMIT or (member.isdir() and member.size): raise ValueError('Runtime member size')
            total += member.size
            if total > EXPANDED_LIMIT: raise ValueError('Runtime expansion budget')
            payload = len(parts) >= 3 and parts[1] == 'rustc' and parts[2:] != ['manifest.in']
            if len(parts) >= 3 and parts[1] != 'rustc': raise ValueError('Unexpected runtime component')
            target = destination.joinpath(*parts[2:]) if payload else None
            if member.isdir():
                if target is not None: target.mkdir(parents=True, exist_ok=True); directory(target)
                continue
            if len(parts) == 2 and parts[1] == 'git-commit-hash':
                if member.size != 40: raise ValueError('Runtime compiler commit size')
                commit = archive.extractfile(member).read(41).decode('ascii')
            elif target is not None:
                target.parent.mkdir(parents=True, exist_ok=True); directory(target.parent)
                digest = hashlib.sha256(); count = 0
                with archive.extractfile(member) as source, target.open('xb') as output:
                    while chunk := source.read(min(65536, member.size-count+1)):
                        count += len(chunk)
                        if count > member.size: raise ValueError('Runtime member grew')
                        digest.update(chunk); output.write(chunk)
                    output.flush(); os.fsync(output.fileno())
                if count != member.size: raise ValueError('Truncated runtime member')
                files[target.relative_to(destination).as_posix()] = {'bytes': count, 'sha256': digest.hexdigest()}
    if commit != COMMIT: raise ValueError('Runtime compiler commit differs')
    for name in ('rustc.exe', 'rustdoc.exe', *REQUIRED_DLLS):
        if 'bin/'+name not in files or not files['bin/'+name]['bytes']: raise ValueError('Missing compiler runtime file: '+name)
    if seal(destination) != files: raise ValueError('Runtime extraction readback differs')
    if file_record(path, ARCHIVE_LIMIT) != archive_record: raise ValueError('Runtime archive changed')
    return {'archive': archive_record, 'component': 'rustc', 'compiler_commit': commit,
            'members': len(seen), 'expanded_bytes': total, 'files': files,
            'required_dlls': list(REQUIRED_DLLS), 'transitive_pe_closure_verified': False}


def seal(root):
    root = directory(root); files = {}; total = 0
    def walk(parent):
        nonlocal total
        for path in sorted(parent.iterdir()):
            info = path.lstat(); plain(info)
            if stat.S_ISDIR(info.st_mode): walk(path)
            else:
                if len(files) >= MEMBER_LIMIT: raise ValueError('Runtime tree count')
                value = file_record(path); total += value['bytes']
                if total > EXPANDED_LIMIT: raise ValueError('Runtime tree bytes')
                files[path.relative_to(root).as_posix()] = value
    walk(root); return files


class MemoryStatus(C.Structure):
    _fields_ = [('length', C.c_uint32), ('load', C.c_uint32),
                *[(name, C.c_uint64) for name in ('total_physical', 'available_physical',
                   'total_pagefile', 'available_pagefile', 'total_virtual', 'available_virtual', 'extended')]]


def memory():
    if os.name != 'nt': raise ValueError('Windows memory observation only')
    fn = C.WinDLL('kernel32', use_last_error=True).GlobalMemoryStatusEx
    fn.argtypes = [C.POINTER(MemoryStatus)]; fn.restype = C.c_int
    value = MemoryStatus(); value.length = C.sizeof(value)
    if not fn(C.byref(value)): raise C.WinError(C.get_last_error())
    result = {name: int(getattr(value, name)) for name in ('total_physical', 'available_physical', 'total_pagefile', 'available_pagefile')}
    if not 0 < result['available_physical'] <= result['total_physical'] or not 0 < result['available_pagefile'] <= result['total_pagefile']:
        raise ValueError('Invalid memory observation')
    return result


@contextmanager
def observed_guard(base, receipt):
    original = base.disk_guard
    values = receipt['resources'] = {'filesystems': {}, 'memory_samples': 0}
    def check(paths):
        for path in paths:
            usage = shutil.disk_usage(path); key = str(Path(path).anchor)
            row = values['filesystems'].setdefault(key, {'total_bytes': usage.total, 'minimum_free_bytes': usage.free})
            row['minimum_free_bytes'] = min(row['minimum_free_bytes'], usage.free)
        current = memory(); values['memory_samples'] += 1
        if 'memory_first' not in values: values['memory_first'] = current
        values['memory_last'] = current
        values['minimum_available_physical_bytes'] = min(values.get('minimum_available_physical_bytes', current['available_physical']), current['available_physical'])
        original(paths)
        if current['available_physical'] < RAM_FLOOR: raise RuntimeError('Available physical RAM floor reached')
    base.disk_guard = check
    try: yield
    finally: base.disk_guard = original


def main():
    import sys
    if os.name != 'nt' or len(sys.argv) != 3: raise ValueError('Fixed Windows runtime phase only')
    work = Path(sys.argv[2])
    if sys.argv[1] == 'download': result = download(work/'rustc.tar.gz')
    elif sys.argv[1] == 'unpack':
        result = unpack(work/'rustc.tar.gz', work/'formatter')
        # Full member map is held locally and sealed, not emitted to bounded logs.
        raw = json.dumps(result, sort_keys=True).encode()
        if len(raw) > 2*1024**2: raise ValueError('Runtime manifest budget')
        with (work/'rustc-runtime.json').open('xb') as out: out.write(raw); out.flush(); os.fsync(out.fileno())
        result = {k: v for k, v in result.items() if k != 'files'}
        result['manifest'] = {'bytes': len(raw), 'sha256': sha(raw)}
    else: raise ValueError('Unknown runtime phase')
    print(json.dumps(result, sort_keys=True), flush=True)


if __name__ == '__main__': main()
