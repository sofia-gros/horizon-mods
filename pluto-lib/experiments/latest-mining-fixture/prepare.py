"""Prepare unchanged mining rules/math and 12 extra latest-official scalar cases.

No build, test, fetch, Mixins or production activation. --resolve is a separate
offline metadata operation; its invocation needs the execution owner's grant.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import signal
import stat
import subprocess
import time
import tomllib

HERE = Path(__file__).resolve().parent
SOURCE = HERE.parent / 'latest-mining-differential'
ROOT = HERE.parents[2]
MOD_PIN = '08afa992d5091e03360f3473ea93fd17aa793987'
MOD_URL = 'https://github.com/sofia-gros/horizon-mods.git'
PIN = 'f4dcf8a2fb4a68b2d28422f4d34d2be2970e26bd'
LOCK = '0bc4234f3001d44df9d46fbb06df331d3f53b1af6b613ec9082baaec3d93c62f'
NAME = 'pluto-latest-mining-differential-tests'
RECEIPT = 'fixture-sources.json'
NATIVE = ('horizon-core', 'horizon-client-core', 'horizon-items', 'horizon-world')

def sha(data):
    return hashlib.sha256(data).hexdigest()

def run(args, cwd=None):
    # This helper is executed as the leader of an outer Runner-owned PGID.
    # Never create a nested session: even SIGKILL of this preparer must leave
    # metadata/Git descendants inside the outer group's bounded teardown.
    if (os.environ.get('PLUTO_FIXTURE_CONTROLLER') != 'latest-mining-differential-v1'
            or os.getpgrp() != os.getpid()):
        raise ValueError('Require the dedicated outer fixture controller PGID')
    process = subprocess.Popen(args, cwd=cwd, text=True, stdout=subprocess.PIPE,
                               stderr=subprocess.PIPE, start_new_session=False)
    try:
        out, err = process.communicate(timeout=60)
        if process.returncode:
            raise ValueError(f'Command failed ({process.returncode}): {args!r}\n{err[-4000:]}')
        return out
    finally:
        # Direct child only. Its descendants stay in our inherited PGID and
        # are always killed/reaped by the outer Runner when this helper exits.
        if process.poll() is None:
            process.kill()
        process.wait(timeout=2)

def tree(root, exclude_git=False):
    """Regular bytes/modes, directories/kinds and links; no link traversal."""
    out = {}
    for directory, directories, files in os.walk(root, followlinks=False):
        for name in sorted(directories + files):
            path = Path(directory) / name
            relative = path.relative_to(root).as_posix()
            if exclude_git and relative == '.git':
                if name in directories:
                    directories.remove(name)
                continue
            mode = path.lstat().st_mode
            if stat.S_ISLNK(mode):
                out[relative] = {'kind': 'link', 'target': os.readlink(path)}
                if name in directories:
                    directories.remove(name)
            elif stat.S_ISDIR(mode):
                out[relative] = {'kind': 'directory'}
            elif stat.S_ISREG(mode):
                out[relative] = {'kind': 'file', 'mode': stat.S_IMODE(mode),
                                 'sha256': sha(path.read_bytes())}
            else:
                raise ValueError('Special file: ' + relative)
    return out

def exact_files(root, expected):
    actual = tree(root)
    if set(actual) != set(expected) or any(v['kind'] != 'file' for v in actual.values()):
        raise ValueError('Frozen file-set/kind differs: ' + str(root))
    for name, digest in expected.items():
        if actual[name]['sha256'] != digest:
            raise ValueError('Frozen source differs: ' + name)
    return actual

def inputs():
    meta = json.loads((HERE / 'inputs.json').read_text())
    if meta['official'] != PIN or meta['official_lock_sha256'] != LOCK:
        raise ValueError('Official pin/lock changed')
    source = exact_files(SOURCE, meta['original_group_sha256'])
    helper = exact_files(HERE, dict(meta['helper_sha256'], **{
        'inputs.json': sha((HERE / 'inputs.json').read_bytes())}))
    original = json.loads((SOURCE / 'inputs.json').read_text())
    names = ['regression_tests::' + n for n in re.findall(r'#\[test\]\s*fn ([a-z0-9_]+)', (SOURCE / 'regression_tests.rs').read_text())]
    if names != meta['additional_tests'] or names != ['regression_tests::'+n for n in original['required_additional_tests']] or len(set(names)) != 12:
        raise ValueError('Additional test inventory changed')
    if len(meta['existing_tests']) != 24 or len(set(meta['expected_tests'])) != 36 or meta['expected_tests'] != meta['existing_tests'] + names:
        raise ValueError('Complete existing/additional inventory changed')
    return meta, {'original_group': source, 'preparer_group': helper}


def clean_official(host):
    remote = run(['git', '-C', str(host), 'config', '--get', 'remote.origin.url']).strip()
    if remote not in {'https://github.com/horizon-rust/horizon.git', 'https://github.com/horizon-rust/horizon'}:
        raise ValueError('Not the authorized official repository')
    if run(['git', '-C', str(host), 'rev-parse', 'HEAD']).strip() != PIN:
        raise ValueError('Official revision differs')
    if run(['git', '-C', str(host), 'status', '--porcelain', '--untracked-files=all', '--ignored=matching']).strip():
        raise ValueError('Official tracked/untracked/ignored source is not clean')
    if sha((host / 'Cargo.lock').read_bytes()) != LOCK:
        raise ValueError('Official lock differs')

def render_sources(host, oracle, meta):
    import importlib.util
    if run(['git', '-C', str(oracle), 'config', '--get', 'remote.origin.url']).strip() != MOD_URL:
        raise ValueError('Not the authorized Mod source repository')
    if run(['git', '-C', str(oracle), 'rev-parse', 'FETCH_HEAD']).strip() != MOD_PIN:
        raise ValueError('Wrong immutable Mod source commit')
    spec = json.loads((SOURCE / 'inputs.json').read_text())
    data = {}
    for path in spec['source_sha256']:
        if path.startswith('official/'):
            file = host / path.removeprefix('official/')
            if file.is_symlink(): raise ValueError('Native source is a link')
            data[path] = file.read_bytes()
        else:
            data[path] = run(['git', '-C', str(oracle), 'show', MOD_PIN + ':' + path]).encode()
    loader = importlib.util.spec_from_file_location('frozen_mining_renderer', SOURCE / 'prepare_sources.py')
    renderer = importlib.util.module_from_spec(loader); loader.loader.exec_module(renderer)
    generated = renderer.render(data, (SOURCE / 'regression_tests.rs').read_bytes())
    if {k: sha(v) for k, v in generated.items()} != meta['generated_sha256']:
        raise ValueError('Generated source differs from frozen render')
    return generated

def official(host, meta):
    clean_official(host)
    for name, digest in meta['native_sha256'].items():
        if sha((host / name).read_bytes()) != digest:
            raise ValueError('Official source differs: ' + name)
    return tree(host, exclude_git=True)

def manifest(host):
    text = ('[package]\nname="' + NAME + '"\nversion="0.0.0"\nedition="2024"\n'
            '[workspace]\n[lib]\npath="lib.rs"\n[dependencies]\nserde_json="1"\n')
    # Same actual items/core/world dependency family as the successful item22
    # fixture, with no test-support feature, invented host or replacement crate.
    for name in ('horizon-items', 'horizon-client-core', 'horizon-world'):
        text += name + '={path=' + json.dumps(str(host / 'crates' / name)) + '}\n'
    return text

def fixture_tree(output):
    actual = tree(output)
    meta, _ = inputs()
    expected = set(meta['generated_sha256']) | {'Cargo.toml', 'Cargo.lock', RECEIPT}
    if set(actual) != expected or any(v['kind'] != 'file' for v in actual.values()):
        raise ValueError('Fixture file-set/kind differs')
    actual.pop(RECEIPT)
    return actual

def prepare(host, oracle, output):
    host = host.resolve()
    oracle = oracle.resolve()
    output = output.absolute()
    if output.exists() or output.is_symlink():
        raise ValueError('Output must be a new fixture directory')
    if output.resolve().is_relative_to(host) or output.resolve().is_relative_to(HERE.parents[2]):
        raise ValueError('Fixture may not modify official or Mod source')
    meta, source = inputs()
    native = official(host, meta)
    output.mkdir(parents=True, exist_ok=False)
    for name, data in render_sources(host, oracle, meta).items():
        (output / name).write_bytes(data)
    (output / 'Cargo.toml').write_text(manifest(host))
    (output / 'Cargo.lock').write_bytes((host / 'Cargo.lock').read_bytes())
    evidence = {'official': PIN, 'host': str(host), 'oracle': str(oracle), 'mod_revision': MOD_PIN, 'source': source,
                'official_tree': native, 'expected_tests': meta['expected_tests'],
                'resolved': False, 'scope': '24 unchanged policy tests plus 12 scalar cases with actual native dependencies; no Game adapter or actor authority proof'}
    evidence['fixture_tree'] = tree(output)
    (output / RECEIPT).write_text(json.dumps(evidence, indent=2) + '\n')
    verify(output)
    return evidence

def verify(output, resolved=False, permit_adapted_lock=False):
    meta, source = inputs()
    evidence = json.loads((output / RECEIPT).read_text())
    host = Path(evidence['host'])
    if evidence['official'] != PIN or evidence['source'] != source:
        raise ValueError('Source identity drift')
    if evidence['expected_tests'] != meta['expected_tests']:
        raise ValueError('Test inventory drift')
    if official(host, meta) != evidence['official_tree']:
        raise ValueError('Official source/link/file-set drift')
    actual = fixture_tree(output)
    expected = dict(evidence['fixture_tree'])
    if permit_adapted_lock:
        actual.pop('Cargo.lock'); expected.pop('Cargo.lock')
    if actual != expected:
        raise ValueError('Fixture source/lock drift')
    if evidence['mod_revision'] != MOD_PIN: raise ValueError('Mod source identity drift')
    for name, data in render_sources(host, Path(evidence['oracle']), meta).items():
        if (output / name).read_bytes() != data: raise ValueError('Generated policy/native fragment drift: ' + name)
    if (output / 'Cargo.toml').read_text() != manifest(host):
        raise ValueError('Generated actual-host manifest changed')
    if resolved and not evidence['resolved']:
        raise ValueError('Dependency identity has not been resolved')
    return evidence

def validate_dependencies(host, output, metadata, original, adapted):
    identity = lambda p: (p['name'], p['version'], p.get('source'), p.get('checksum'))
    allowed = {identity(p) for p in original['package']}
    actual = {identity(p) for p in adapted['package']}
    own = (NAME, '0.0.0', None, None)
    if actual - allowed - {own} or own not in actual or len(actual) != len(adapted['package']):
        raise ValueError('Dependency lock identity escaped official lock')
    package_ids = []
    for package in metadata['packages']:
        path = Path(package['manifest_path']).resolve()
        package_ids.append(package['id'])
        if package['name'] == NAME:
            if path != output / 'Cargo.toml' or package['source'] is not None:
                raise ValueError('Wrong fixture package identity')
        elif package['source'] is None:
            if not path.is_relative_to(host / 'crates'):
                raise ValueError('Native dependency escaped official crate tree')
        if (package['name'], package['version'], package.get('source')) not in {p[:3] for p in actual}:
            raise ValueError('Metadata package is not in adapted official lock')
    if len(set(package_ids)) != len(package_ids):
        raise ValueError('Duplicate metadata package identity')
    for name in (*NATIVE, NAME):
        matches = [p for p in metadata['packages'] if p['name'] == name]
        expected = output / 'Cargo.toml' if name == NAME else host / 'crates' / name / 'Cargo.toml'
        if len(matches) != 1 or Path(matches[0]['manifest_path']).resolve() != expected or matches[0]['source'] is not None:
            raise ValueError('Expected one actual native/fixture package: ' + name)
    own_id = next(p['id'] for p in metadata['packages'] if p['name'] == NAME)
    if metadata['workspace_members'] != [own_id]:
        raise ValueError('Fixture workspace identity differs')

def resolve(output):
    output = output.resolve()
    evidence = verify(output)
    if evidence['resolved']:
        raise ValueError('Already resolved; use --verify rather than resolving again')
    host = Path(evidence['host'])
    metadata = json.loads(run(['cargo', 'metadata', '--offline', '--format-version=1',
                               '--manifest-path', str(output / 'Cargo.toml')], cwd=output))
    verify(output, permit_adapted_lock=True)
    validate_dependencies(host, output, metadata,
                          tomllib.loads((host / 'Cargo.lock').read_text()),
                          tomllib.loads((output / 'Cargo.lock').read_text()))
    evidence['fixture_tree'] = fixture_tree(output)
    evidence['resolved'] = True
    evidence['dependencies'] = [{'name': p['name'], 'manifest_path': p['manifest_path'], 'source': p['source']} for p in metadata['packages']]
    (output / RECEIPT).write_text(json.dumps(evidence, indent=2) + '\n')
    verify(output, resolved=True)
    return evidence

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--official', type=Path)
    parser.add_argument('--oracle', type=Path)
    parser.add_argument('--output', type=Path, required=True)
    group = parser.add_mutually_exclusive_group()
    group.add_argument('--verify', action='store_true')
    group.add_argument('--resolve', action='store_true')
    args = parser.parse_args()
    if args.verify: evidence = verify(args.output.resolve(), resolved=True)
    elif args.resolve: evidence = resolve(args.output)
    else:
        if not args.official or not args.oracle: parser.error('--official and --oracle are required for preparation')
        evidence = prepare(args.official, args.oracle, args.output)
    print(json.dumps({'prepared': True, 'resolved': evidence['resolved'], 'expected_tests': evidence['expected_tests']}))

if __name__ == '__main__': main()
