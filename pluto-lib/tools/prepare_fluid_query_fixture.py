#!/usr/bin/env python3
"""Prepare an exact-official-types decoder fixture, without compiling it.

By default tests exercise the real BlockRegistry decoder only. The optional
--with-world-view mode adds synthetic minimal resources through native loaders
and in-memory World sampling. Neither mode uses production game assets,
exercises Game wrappers, or proves contact gameplay.
"""
import argparse
import hashlib
import json
import re
from pathlib import Path
import shutil
import subprocess
import tomllib
from prepare_render_fixture import check_identities, clean_official

FIXTURE_IDENTITY = ('pluto-world-fluid-query', '0.0.0', None, None)
API_IDENTITY = ('pluto-api', '0.1.0', None, None)


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def api_inputs(api):
    manifest = tomllib.loads((api / 'Cargo.toml').read_text())
    if (manifest['package']['name'], manifest['package']['version'], None, None) != API_IDENTITY:
        raise ValueError('Require the exact actual Pluto API package identity')
    if (api / 'build.rs').exists() or manifest['package'].get('build') not in (None, False):
        raise ValueError('Unexpected build script in fixture API dependency')
    paths = [api / 'Cargo.toml', *sorted((api / 'src').rglob('*.rs'))]
    if not (api / 'src/lib.rs').is_file() or any(p.is_symlink() for p in paths):
        raise ValueError('Require regular actual API sources')
    return {p.relative_to(api).as_posix(): digest(p) for p in paths}


def check_lock(host_lock, fixture_lock):
    # The only additional dependency is the actual, source-hashed Pluto crate.
    # The fixture identity is separately required by the shared strict guard.
    api = {'name': API_IDENTITY[0], 'version': API_IDENTITY[1]}
    check_identities({'package': [*host_lock['package'], api]}, fixture_lock, FIXTURE_IDENTITY)
    if not any((p.get('name'), p.get('version'), p.get('source'), p.get('checksum')) == API_IDENTITY
               for p in fixture_lock['package']):
        raise ValueError('The fixture must link the actual Pluto API crate')


def verify(output):
    evidence = json.loads((output / 'fixture-sources.json').read_text())
    host = Path(evidence['horizon_source'])
    clean_official(host, evidence['horizon_revision'])
    if digest(host / 'Cargo.lock') != evidence['official_lock_sha256']:
        raise ValueError('Official lock changed after preparation')
    for name, expected in evidence['sources_sha256'].items():
        if digest(output / name) != expected:
            raise ValueError(f'Fixture input changed: {name}')
    if api_inputs(Path(evidence['api_source'])) != evidence['api_sources_sha256']:
        raise ValueError('Actual API crate changed after preparation')
    for name, expected in evidence.get('official_fixture_builder_inputs', {}).items():
        if digest(host / name) != expected:
            raise ValueError(f'Official fixture builder input changed: {name}')
    check_lock(tomllib.loads((host / 'Cargo.lock').read_text()),
               tomllib.loads((output / 'Cargo.lock').read_text()))
    return evidence


def required_blocks(source):
    match = re.search(r'named_blocks! \{(.*?)\n\}', source, re.S)
    if match is None:
        raise ValueError('Missing exact official Named block declaration')
    names = []
    for line in match[1].splitlines():
        if not line.strip() or line.lstrip().startswith('//'):
            continue
        row = re.fullmatch(r'\s*[a-z_][a-z0-9_]* = "([a-z0-9_]+)",\s*', line)
        if row is None:
            raise ValueError('Unexpected Named block declaration syntax')
        name = 'minecraft:' + row[1]
        if name not in names:  # Several native fields may alias the same block.
            names.append(name)
    if not names or names[0] != 'minecraft:air':
        raise ValueError('Invalid official Named block identities/order')
    return names


def prepare(host, output, with_world_view=False):
    root = Path(__file__).resolve().parents[1]
    host = host.resolve(); output = output.resolve()
    native = root / 'mixins/pluto_native_creative'
    pin = tomllib.loads((native / 'horizon-mixin.toml').read_text())['compatibility']['horizon_revision']
    clean_official(host, pin)
    api = (native / 'api').resolve(); api_hashes = api_inputs(api)
    if output.exists():
        raise ValueError('Fixture output must not already exist')
    output.mkdir(parents=True)
    shutil.copyfile(native / 'world-fluid-query.rs', output / 'world-fluid-query.rs')
    sources = ['world-fluid-query.rs', 'Cargo.toml', 'lib.rs']
    if with_world_view:
        shutil.copyfile(root / 'tests/native/world-fluid-view.rs', output / 'world-fluid-view.rs')
        names = required_blocks((host / 'crates/horizon-world/src/level/data.rs').read_text())
        (output / 'required-blocks.json').write_text(json.dumps(names, indent=2) + '\n')
        sources += ['world-fluid-view.rs', 'required-blocks.json']
    (output / 'Cargo.toml').write_text(
        '[package]\nname = "pluto-world-fluid-query"\nversion = "0.0.0"\nedition = "2024"\n'
        '[workspace]\n[lib]\npath = "lib.rs"\n[dependencies]\n'
        'horizon-client = { package = "horizon-client-render", path = '
        + json.dumps(str(host / 'crates/horizon-client-render')) + ', default-features = false }\n'
        + 'horizon-world = { path = ' + json.dumps(str(host / 'crates/horizon-world')) + ' }\n'
        + 'pluto-api = { path = ' + json.dumps(str(api)) + ' }\n'
        + ('serde_json = \"1\"\n' if with_world_view else ''))
    with (output / 'Cargo.toml').open('a') as manifest:
        manifest.write('\n[patch.crates-io]\n')
        for name, entry in tomllib.loads((host / 'Cargo.toml').read_text()).get('patch', {}).get('crates-io', {}).items():
            if set(entry) != {'path'}:
                raise ValueError(f'Unsupported official patch: {name}')
            path = (host / entry['path']).resolve()
            if not path.is_relative_to(host):
                raise ValueError(f'Official patch escapes checkout: {name}')
            manifest.write(json.dumps(name) + ' = { path = ' + json.dumps(str(path)) + ' }\n')
    (output / 'lib.rs').write_text('#![forbid(unsafe_code)]\nmod query { include!("world-fluid-query.rs"); }\n'
        + ('#[cfg(test)] mod world_view_tests { include!("world-fluid-view.rs"); }\n' if with_world_view else ''))
    (output / 'Cargo.lock').write_bytes((host / 'Cargo.lock').read_bytes())
    subprocess.run(['cargo', 'metadata', '--offline', '--manifest-path', str(output / 'Cargo.toml'),
                    '--format-version', '1'], check=True, stdout=subprocess.DEVNULL)
    evidence = {'horizon_source': str(host), 'horizon_revision': pin,
        'official_lock_sha256': digest(host / 'Cargo.lock'),
        'api_source': str(api), 'api_sources_sha256': api_hashes,
        'sources_sha256': {name: digest(output / name) for name in sources},
        'with_minimal_world_view_fixture': with_world_view,
        'scope': ('Actual native view against synthetic minimal resources validated by real official loaders. No production assets, Game wrappers, actor effects or gameplay.' if with_world_view else
                  'Actual native view types and actual Pluto API crate. Three decoder tests only; no native world sampling, Game wrappers, effects or gameplay.')}
    if with_world_view:
        evidence['official_fixture_builder_inputs'] = {name: digest(host / name) for name in (
            'crates/horizon-world/src/level/data.rs', 'crates/horizon-world/tests/collision_data.rs')}
    (output / 'fixture-sources.json').write_text(json.dumps(evidence, indent=2) + '\n')
    verify(output)
    print(output)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--horizon-source', type=Path)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--verify-lock', action='store_true')
    parser.add_argument('--with-world-view', action='store_true', help='Add synthetic resource-backed native View tests, not production asset parity')
    args = parser.parse_args()
    if args.verify_lock:
        verify(args.output)
        print('Exact official dependencies, actual API crate and query sources verified')
    elif args.horizon_source is None:
        parser.error('--horizon-source is required when preparing a fixture')
    else:
        prepare(args.horizon_source, args.output, args.with_world_view)
