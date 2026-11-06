"""Prepare the four unchanged official Items/default-lifetime tests.

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
SOURCE = HERE.parent / 'item-name-default-lifetime'
PIN = '624c4f6265c02296fbe8a80f029dd0d987fb94c0'
LOCK = '463913123985e344f2889e5beae65982773e6dbfacdcac7a916bcec522a5bd31'
NAME = 'pluto-item-name-default-lifetime-tests'
RECEIPT = 'fixture-sources.json'
NATIVE = ('horizon-core', 'horizon-client-core', 'horizon-items', 'horizon-world')

def sha(data):
    return hashlib.sha256(data).hexdigest()

def run(args, cwd=None):
    # Even metadata has a bounded process group. Compilation is never invoked.
    process = subprocess.Popen(args, cwd=cwd, text=True, stdout=subprocess.PIPE,
                               stderr=subprocess.PIPE, start_new_session=True)
    try:
        out, err = process.communicate(timeout=60)
        if process.returncode:
            raise ValueError(f'Command failed ({process.returncode}): {args!r}\n{err[-4000:]}')
        return out
    finally:
        try:
            os.killpg(process.pid, signal.SIGKILL)
        except ProcessLookupError:
            pass
        process.wait(timeout=2)
        deadline = time.monotonic() + 1
        while True:
            try:
                os.killpg(process.pid, 0)
            except ProcessLookupError:
                break
            if time.monotonic() >= deadline:
                raise ValueError('Own metadata/Git process group remains after bounded teardown')
            time.sleep(0.02)

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
    names = re.findall(r'#\[test\]\s*fn ([a-z0-9_]+)', (SOURCE / 'catalog_tests.rs').read_text())
    if names != meta['expected_tests'] or len(set(names)) != 4:
        raise ValueError('Four-test inventory changed')
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
    expected = {'lib.rs', 'Cargo.toml', 'Cargo.lock', RECEIPT}
    if set(actual) != expected or any(v['kind'] != 'file' for v in actual.values()):
        raise ValueError('Fixture file-set/kind differs')
    actual.pop(RECEIPT)
    return actual

def prepare(host, output):
    host = host.resolve()
    output = output.absolute()
    if output.exists() or output.is_symlink():
        raise ValueError('Output must be a new fixture directory')
    if output.resolve().is_relative_to(host) or output.resolve().is_relative_to(HERE.parents[2]):
        raise ValueError('Fixture may not modify official or Mod source')
    meta, source = inputs()
    native = official(host, meta)
    output.mkdir(parents=True, exist_ok=False)
    (output / 'lib.rs').write_bytes((SOURCE / 'catalog_tests.rs').read_bytes())
    (output / 'Cargo.toml').write_text(manifest(host))
    (output / 'Cargo.lock').write_bytes((host / 'Cargo.lock').read_bytes())
    evidence = {'official': PIN, 'host': str(host), 'source': source,
                'official_tree': native, 'expected_tests': meta['expected_tests'],
                'resolved': False, 'scope': 'Four unmodified native catalog/default lifetime cases; no production connection'}
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
    if (output / 'lib.rs').read_bytes() != (SOURCE / 'catalog_tests.rs').read_bytes():
        raise ValueError('Original test body changed')
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
    parser.add_argument('--output', type=Path, required=True)
    group = parser.add_mutually_exclusive_group()
    group.add_argument('--verify', action='store_true')
    group.add_argument('--resolve', action='store_true')
    args = parser.parse_args()
    if args.verify: evidence = verify(args.output.resolve(), resolved=True)
    elif args.resolve: evidence = resolve(args.output)
    else:
        if not args.official: parser.error('--official is required for preparation')
        evidence = prepare(args.official, args.output)
    print(json.dumps({'prepared': True, 'resolved': evidence['resolved'], 'expected_tests': evidence['expected_tests']}))

if __name__ == '__main__': main()
