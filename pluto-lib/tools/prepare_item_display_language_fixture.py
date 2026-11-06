#!/usr/bin/env python3
"""Prepare a small actual-native item display fixture; never compile implicitly.

Preparation only reads the pinned official checkout and writes a new bounded
fixture. --resolve is a separate Cargo-metadata operation requiring its own
execution grant. No host edits, clone, download, apply or registry creation.
"""
import argparse
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import tomllib

ROOT = Path(__file__).resolve().parents[1]
PIN = '624c4f6265c02296fbe8a80f029dd0d987fb94c0'
NAME = 'pluto-item-display-language-tests'
OFFICIAL_REMOTES = {'https://github.com/horizon-rust/horizon.git',
                    'https://github.com/horizon-rust/horizon'}

def digest(data):
    return hashlib.sha256(data).hexdigest()

def run(args, cwd=None):
    return subprocess.check_output(args, cwd=cwd, text=True, timeout=60)

def clean_official(host):
    if run(['git', '-C', str(host), 'config', '--get', 'remote.origin.url']).strip() not in OFFICIAL_REMOTES:
        raise ValueError('Not the authorized official repository')
    if run(['git', '-C', str(host), 'rev-parse', 'HEAD']).strip() != PIN:
        raise ValueError('Official revision mismatch')
    if run(['git', '-C', str(host), 'status', '--porcelain', '--untracked-files=no']).strip():
        raise ValueError('Official tracked source is not clean')

def section(text, start, end):
    if text.count(start) != 1 or text.count(end) != 1:
        raise ValueError('Source boundaries are not unique')
    a, b = text.index(start), text.index(end)
    if b <= a:
        raise ValueError('Source boundary order changed')
    return text[a:b]

def files(root):
    out = {}
    for path in root.rglob('*'):
        if path.is_symlink():
            raise ValueError('Symlink in fixture')
        if path.is_file():
            out[path.relative_to(root).as_posix()] = digest(path.read_bytes())
        elif not path.is_dir():
            raise ValueError('Special file in fixture')
    return out

def prepare(host, output, language, author, example):
    host, output = host.resolve(), output.absolute()
    clean_official(host)
    manifest = json.loads((ROOT / 'docs/item-display-language-inputs-20261106.json').read_text())
    for relative, expected in manifest['official_sha256'].items():
        if digest((host / relative).read_bytes()) != expected:
            raise ValueError('Official source hash mismatch: ' + relative)
    for route in manifest['source_only_routes']:
        value = section((host / route['source']).read_text(), route['start'], route['end'])
        if digest(value.encode()) != route['sha256']:
            raise ValueError('Source-only display route changed')
    for path, role in [(language, 'language'), (author, 'author'), (example, 'example')]:
        if path.is_symlink() or not path.is_file() or digest(path.read_bytes()) != manifest['adapters'][role]['sha256']:
            raise ValueError('Frozen adapter mismatch: ' + role)
    source = ROOT / 'tests/native/item_display_language.rs'
    if digest(source.read_bytes()) != manifest['fixture_sha256']:
        raise ValueError('Fixture source changed')
    output.mkdir(parents=True, exist_ok=False)
    shutil.copyfile(source, output / 'lib.rs')
    shutil.copyfile(language, output / 'language.rs')
    shutil.copyfile(author, output / 'language-text.rs')
    shutil.copyfile(example, output / 'language_text_example.rs')
    shutil.copyfile(host / 'Cargo.lock', output / 'Cargo.lock')
    records = []
    def take(relative, start, end, destination):
        value = section((host / relative).read_text(), start, end)
        records.append(dict(source=relative, start=start, end=end,
                            destination=destination, sha256=digest(value.encode())))
        return value
    names = take('crates/horizon-client/src/creative.rs',
                 '#[derive(Default)]\npub struct Names',
                 "/// `CreativeModeTabs`' key", 'native_names.rs')
    (output / 'native_names.rs').write_text('use std::{collections::HashMap, fs, path::Path};\n' + names)
    styled = take('crates/horizon-client/src/item_tooltip.rs',
                  'pub fn styled_name(', '#[cfg(test)]\nmod tests', 'native_styled_name.rs')
    (output / 'native_styled_name.rs').write_text('use horizon_items::items::ItemStack;\n' + styled)
    cargo = '[package]\nname="' + NAME + '"\nversion="0.0.0"\nedition="2024"\n[workspace]\n[lib]\npath="lib.rs"\n[dependencies]\nserde_json="1"\n'
    for name in ['horizon-items', 'horizon-client-core', 'horizon-world']:
        cargo += name + '={path=' + json.dumps(str(host / 'crates' / name)) + '}\n'
    (output / 'Cargo.toml').write_text(cargo)
    evidence = dict(official=PIN, official_source=str(host), official_sha256=manifest['official_sha256'],
                    adapters=manifest['adapters'], source_only_routes=manifest['source_only_routes'], fragments=records,
                    expected_tests=22, result_groups=manifest['result_groups'], execution_strategy=manifest['execution_strategy'], new_item_display_tests=8, inherited_language_tests=14,
                    source_files=files(output), resolved=False)
    (output / 'fixture-sources.json').write_text(json.dumps(evidence, indent=2) + '\n')
    return evidence

def verify(output, require_resolved=False):
    evidence = json.loads((output / 'fixture-sources.json').read_text())
    host = Path(evidence['official_source'])
    clean_official(host)
    for relative, expected in evidence['official_sha256'].items():
        if digest((host / relative).read_bytes()) != expected:
            raise ValueError('Official drift: ' + relative)
    expected_files = dict(evidence['source_files'])
    actual_files = files(output)
    actual_files.pop('fixture-sources.json')
    if actual_files != expected_files:
        raise ValueError('Fixture file-set or source drift')
    for record in evidence['fragments']:
        value = section((host / record['source']).read_text(), record['start'], record['end'])
        if digest(value.encode()) != record['sha256'] or value not in (output / record['destination']).read_text():
            raise ValueError('Official fragment drift')
    for route in evidence['source_only_routes']:
        value = section((host / route['source']).read_text(), route['start'], route['end'])
        if digest(value.encode()) != route['sha256']:
            raise ValueError('Source-only route drift')
    if require_resolved and not evidence['resolved']:
        raise ValueError('Lock and real path identity not resolved yet')
    return evidence

def resolve(output):
    evidence = verify(output)
    host = Path(evidence['official_source'])
    # First metadata is permitted to adapt only this copied lock; it cannot compile.
    metadata = json.loads(run(['cargo', 'metadata', '--offline', '--format-version=1'], cwd=output))
    original = tomllib.loads((host / 'Cargo.lock').read_text())
    resolved = tomllib.loads((output / 'Cargo.lock').read_text())
    identity = lambda p: (p['name'], p['version'], p.get('source'), p.get('checksum'))
    allowed = {identity(p) for p in original['package']}
    local = (NAME, '0.0.0', None, None)
    got = {identity(p) for p in resolved['package']}
    if got - allowed - {local} or local not in got:
        raise ValueError('Nonofficial locked dependency identity')
    for package in metadata['packages']:
        path = Path(package['manifest_path']).resolve()
        if package['name'] == NAME:
            if path != (output / 'Cargo.toml').resolve() or package['source'] is not None:
                raise ValueError('Fixture identity mismatch')
        elif package['source'] is None and not path.is_relative_to(host / 'crates'):
            raise ValueError('Native dependency escaped official checkout')
    for name in ['horizon-items', 'horizon-client-core', 'horizon-world']:
        matches = [p for p in metadata['packages'] if p['name'] == name]
        if len(matches) != 1 or Path(matches[0]['manifest_path']).resolve() != host / 'crates' / name / 'Cargo.toml':
            raise ValueError('Expected one actual official package: ' + name)
    evidence['source_files']['Cargo.lock'] = digest((output / 'Cargo.lock').read_bytes())
    evidence['resolved'] = True
    evidence['dependency_packages'] = [{'name': p['name'], 'manifest_path': p['manifest_path'], 'source': p['source']} for p in metadata['packages']]
    (output / 'fixture-sources.json').write_text(json.dumps(evidence, indent=2) + '\n')
    verify(output, require_resolved=True)
    return evidence

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--official', type=Path)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--language', type=Path)
    parser.add_argument('--author', type=Path)
    parser.add_argument('--example', type=Path)
    group = parser.add_mutually_exclusive_group()
    group.add_argument('--verify', action='store_true')
    group.add_argument('--resolve', action='store_true')
    args = parser.parse_args()
    if args.verify:
        value = verify(args.output, require_resolved=True)
    elif args.resolve:
        value = resolve(args.output)
    else:
        if not all([args.official, args.language, args.author, args.example]):
            parser.error('Preparation requires official, language, author and example inputs')
        value = prepare(args.official, args.output, args.language, args.author, args.example)
    print(json.dumps({'prepared': True, 'resolved': value['resolved'], 'expected_tests': value['expected_tests']}))

if __name__ == '__main__':
    main()
