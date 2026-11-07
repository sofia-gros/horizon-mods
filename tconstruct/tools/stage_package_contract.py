"""Bind explicit full-composition archives to the exact staged member bytes.

Delegates the contract schema to the reviewed Recipe guard. No planner, compiler,
formatter or subprocess is started here. Reads require quiescent ordinary files;
identity rechecks detect ordinary drift, not adversarial filesystem races.
"""
import importlib.util
import json
import os
from pathlib import Path
import re
import stat

import prepare_playtest_bundles as bundles
from playtest_pluto_inputs import directory, plain, read_signature, require_opened_file

IDS = frozenset({'tconstruct_native', 'pluto_native_creative'})
MAX_CONTRACT = 1024 * 1024


def bounded_file(path, limit):
    path = Path(path)
    if not path.is_absolute(): raise ValueError('Package input must be an absolute path')
    directory(path.parent)
    before = path.lstat(); plain(before)
    if not stat.S_ISREG(before.st_mode) or before.st_size > limit:
        raise ValueError('Package input is not a bounded regular file')
    with path.open('rb') as stream:
        opened = os.fstat(stream.fileno()); require_opened_file(path, before, opened)
        raw = stream.read(limit + 1)
        after_handle = os.fstat(stream.fileno()); plain(after_handle)
        if read_signature(opened) != read_signature(after_handle): raise ValueError('Package input changed during read')
    after = path.lstat(); plain(after)
    if len(raw) > limit or len(raw) != before.st_size or read_signature(before) != read_signature(after):
        raise ValueError('Package input changed after read')
    return raw


def archive_members(archives, expected_sha256):
    if set(archives) != IDS or set(expected_sha256) != IDS:
        raise ValueError('Explicit full composition requires exactly both archive identities and hashes')
    if len({str(Path(p)) for p in archives.values()}) != 2:
        raise ValueError('Full composition archives must be distinct')
    files = []
    for identity in sorted(IDS):
        expected = expected_sha256[identity]
        if not isinstance(expected, str) or re.fullmatch('[0-9a-f]{64}', expected) is None:
            raise ValueError('Expected package SHA256 is required')
        raw = bounded_file(archives[identity], bundles.MAX_ARCHIVE)
        if bundles.sha(raw) != expected: raise ValueError('Archive differs from caller-verified package hash')
        values, _ = bundles.inspect(raw, identity)
        files.append(values)
    return bundles.combine(files)[1]['member_sha256']


def verify_members(expected, mods, pluto, combined=None):
    values = {name: bundles.sha(raw) for name, raw in bundles.disk_members(mods).items()}
    for name, raw in bundles.disk_members(pluto).items():
        key = 'pluto_native_creative/' + name
        if key in values: raise ValueError('Duplicate staged package member')
        values[key] = bundles.sha(raw)
    if values != expected: raise ValueError('Expanded packages differ from the complete verified archives')
    if combined is not None:
        actual = {name: bundles.sha(raw) for name, raw in bundles.disk_members(combined).items()}
        if actual != expected: raise ValueError('Copied complete composition differs from verified archives')


def prepare(checker_path, archives, expected_sha256, mods, pluto):
    expected = archive_members(archives, expected_sha256)
    verify_members(expected, mods, pluto)
    spec = importlib.util.spec_from_file_location('stage_recipe_package_contract', checker_path)
    checker = importlib.util.module_from_spec(spec); spec.loader.exec_module(checker)
    # The shared checker owns schema, archive/manifest compatibility and full mode.
    contract = checker.make_package_contract('full-tconstruct', archives)
    observed = {p['mod_id']: (p['archive'], p['archive_sha256']) for p in contract['packages']}
    wanted = {identity: (str(archives[identity]), expected_sha256[identity]) for identity in IDS}
    if len(contract['packages']) != 2 or observed != wanted or contract['mode'] != 'full-tconstruct':
        raise ValueError('Shared package contract does not bind the explicit full archives')
    raw = (json.dumps(contract, sort_keys=True, indent=2) + '\n').encode('utf-8')
    if len(raw) > MAX_CONTRACT: raise ValueError('Package contract exceeds byte budget')
    if archive_members(archives, expected_sha256) != expected: raise ValueError('Archive changed during contract creation')
    verify_members(expected, mods, pluto)
    return raw, expected


def save(path, raw):
    directory(path.parent)
    if len(raw) > MAX_CONTRACT: raise ValueError('Package contract exceeds byte budget')
    with path.open('xb') as stream:
        stream.write(raw); stream.flush(); os.fsync(stream.fileno())
    if bounded_file(path, MAX_CONTRACT) != raw: raise ValueError('Package contract write did not preserve exact bytes')


def verify(path, raw, expected_members, archives, expected_sha256, mods, pluto, combined):
    if bounded_file(path, MAX_CONTRACT) != raw: raise ValueError('Package contract changed')
    if archive_members(archives, expected_sha256) != expected_members: raise ValueError('Package member seal changed')
    verify_members(expected_members, mods, pluto, combined)
