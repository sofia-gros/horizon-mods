#!/usr/bin/env python3
"""In-memory latest terrain oracle/profile/reload composition; no native tools."""
import argparse
import hashlib
import json
from pathlib import Path
import re

HERE = Path(__file__).resolve().parent
HORIZON = 'f4dcf8a2fb4a68b2d28422f4d34d2be2970e26bd'
MIXINS = '5a64af938ec86b00d53d16a408aabc0038fb3e57'
MARKER = b'// INSERT EXACT NATIVE BASELINE'
PROFILES = ('profile', 'reload')
FRAGMENTS = ('experiments/terrain-cpu-profile/profile.rs',
             'experiments/latest-terrain-reload/native.rs')
TEMPLATE = 'experiments/terrain-material-cpu/fixture.rs'
META = 'tools/terrain-cell-source.json'
MAX_SOURCE = 2 * 1024 * 1024
MAX_FRAGMENT = 32768
MAX_OUTPUT = 256 * 1024


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def require(ok, message):
    if not ok:
        raise ValueError(message)


def read(root, path, digest):
    relative = Path(path)
    require(not relative.is_absolute() and '..' not in relative.parts, 'unsafe source path')
    source = (root / relative).absolute()
    require(not any(part.is_symlink() for part in (source, *source.parents)),
            'symlink source component: ' + path)
    require(source.is_file() and
            source.stat().st_size <= MAX_SOURCE, 'invalid selected source: ' + path)
    # A sealed quiescent source tree is supplied by the outer CI controller.
    with source.open('rb') as handle:
        raw = handle.read(MAX_SOURCE + 1)
    require(len(raw) <= MAX_SOURCE and sha(raw) == digest, 'selected source drift: ' + path)
    return raw


def boundary(kind, end=False):
    require(kind in PROFILES, 'unknown extension')
    return ('    // ' + ('END' if end else 'BEGIN') +
            ' EXCLUDED LATEST TERRAIN ' + kind.upper() + '\n').encode()


def add(template, fragment, kind):
    require(template.count(MARKER) == 1, 'ambiguous native marker')
    require(0 < len(fragment) <= MAX_FRAGMENT and MARKER not in fragment,
            'invalid fragment size/marker')
    for name in PROFILES:
        require(boundary(name) not in fragment and boundary(name, True) not in fragment,
                'fragment contains a reserved boundary')
    require(boundary(kind) not in template and boundary(kind, True) not in template,
            'repeated extension')
    needle = b'    ' + MARKER + b'\n'
    require(template.count(needle) == 1, 'original indentation changed')
    result = template.replace(needle, boundary(kind) + fragment + b'\n' +
                              boundary(kind, True) + needle, 1)
    require(remove(result, kind) == template, 'extension inverse changed')
    require(len(result) <= MAX_OUTPUT, 'composed source size limit')
    return result


def remove(value, kind):
    begin, end = boundary(kind), boundary(kind, True)
    require(value.count(begin) == value.count(end) == 1, 'ambiguous inverse boundary')
    first, last = value.index(begin), value.index(end) + len(end)
    require(first < last - len(end), 'reversed inverse boundary')
    return value[:first] + value[last:]


def baseline(template, native_source, meta):
    require(sha(native_source) == meta['file_sha256'], 'native mesher file changed')
    text = native_source.decode('utf-8')
    require(text.count(meta['start']) == 1, 'native function start changed')
    start = text.index(meta['start'])
    require(text[start:].count(meta['end']) == 1, 'native function end changed')
    original = text[start:text.index(meta['end'], start)].rstrip()
    require(sha(original.encode()) == meta['function_sha256'], 'native function changed')
    # Exact historical receiver/name transform. No adapter body or API is modeled.
    native = original.replace('    pub fn mesh_reusing(', 'fn native_baseline(', 1)
    native = native.replace('&mut self,', "mesher: &mut Mesher<'_>,", 1)
    native = re.sub(r'\bself\b', 'mesher', native).encode()
    require(template.count(MARKER) == 1, 'ambiguous baseline marker')
    return template.replace(MARKER, native, 1)


def compose(mod_root, latest):
    contract_raw = (HERE / 'inputs.json').read_bytes()
    require(len(contract_raw) <= 65536, 'contract size limit')
    config = json.loads(contract_raw)
    require(config['schema'] == 1 and config['horizon'] == HORIZON and
            config['mixins'] == MIXINS, 'wrong latest source contract')
    require(not any(config['execution'].values()), 'source candidate cannot claim execution')
    require(len(config['selected_latest_sources']) == 23 and
            len(config['frozen_mod_sources']) == 18, 'wrong selected source inventory')
    root = Path(mod_root) / 'pluto-lib'
    current = {p: read(root, p, h) for p, h in config['frozen_mod_sources'].items()}
    official = {p: read(Path(latest), p, h) for p, h in config['selected_latest_sources'].items()}
    template = current[TEMPLATE]
    meta = json.loads(current[META])
    generated_original = baseline(template, official[meta['file']], meta)
    composed = template
    for kind, path in zip(PROFILES, FRAGMENTS):
        composed = add(composed, current[path], kind)
    generated = baseline(composed, official[meta['file']], meta)
    inverse = generated
    for kind in reversed(PROFILES):
        inverse = remove(inverse, kind)
    require(inverse == generated_original, 'full native oracle/original fixture not preserved')
    tests = re.findall(rb'#\[test\]\s*fn\s+(\w+)\(', generated)
    require(sorted(x.decode() for x in tests) == sorted(config['expected_tests']) and
            len(tests) == len(set(tests)) == 3, 'wrong Rust source test inventory')
    require(b'#[ignore]' not in generated and b'#![' not in generated,
            'unsupported/ignored test source')
    cases = re.findall(rb'println!\("RELOAD_CASE (\w+) ok"\)', current[FRAGMENTS[1]])
    require([x.decode() for x in cases] == config['expected_reload_cases'], 'reload cases changed')
    require(len(generated) <= MAX_OUTPUT, 'generated source size limit')
    # Reread all selected inputs, not just the mechanically extracted function.
    require(current == {p: read(root, p, h) for p, h in config['frozen_mod_sources'].items()},
            'Mod inputs changed during composition')
    require(official == {p: read(Path(latest), p, h) for p, h in config['selected_latest_sources'].items()},
            'official selected inputs changed during composition')
    require(contract_raw == (HERE / 'inputs.json').read_bytes(), 'contract changed during composition')
    result = {'status': 'source_composed_only', 'horizon': HORIZON, 'mixins': MIXINS,
              'selected_official_files': len(official), 'pinned_mod_files': len(current),
              'baseline_generated_sha256': sha(generated_original),
              'composed_sha256': sha(generated), 'composed_bytes': len(generated),
              'native_function_sha256': meta['function_sha256'],
              'prepared_rust_tests': [x.decode() for x in tests],
              'prepared_reload_cases': len(cases),
              'compiler_profiles_observed': False, 'compatibility': None,
              'execution': config['execution']}
    return generated, result


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--mod-root', type=Path, required=True)
    parser.add_argument('--latest-selected-source', type=Path, required=True)
    args = parser.parse_args()
    _, evidence = compose(args.mod_root, args.latest_selected_source)
    print(json.dumps(evidence, sort_keys=True))
