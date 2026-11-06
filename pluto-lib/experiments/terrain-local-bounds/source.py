#!/usr/bin/env python3
"""Compose an excluded std-only comparison in memory; never invoke Rust."""
import argparse
import hashlib
import json
from pathlib import Path
import re

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
LIMIT = 128 * 1024
GEOMETRY = 'pluto-lib/mixins/pluto_native_creative/terrain-cell-geometry.rs'
PROVIDERS = 'pluto-lib/mixins/pluto_native_creative/terrain-cell-providers.rs'
START = b'pub(super) fn cell_local(batch: &Batch) -> bool {\n'
END = b'pub(super) fn tint_color('


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def require(ok, message):
    if not ok:
        raise ValueError(message)


def read(root, relative, expected):
    part = Path(relative)
    require(not part.is_absolute() and '..' not in part.parts, 'unsafe input path')
    source = (Path(root) / part).absolute()
    require(not any(p.is_symlink() for p in (source, *source.parents)), 'symlink input component')
    require(source.is_file() and source.stat().st_size <= LIMIT, 'invalid input')
    with source.open('rb') as handle:
        raw = handle.read(LIMIT + 1)
    require(len(raw) <= LIMIT and sha(raw) == expected, 'source drift: ' + relative)
    return raw


def extract(providers):
    require(providers.count(START) == providers.count(END) == 1, 'ambiguous original predicate')
    start, end = providers.index(START), providers.index(END)
    require(start < end, 'reversed original predicate')
    result = providers[start:end]
    require(result.endswith(b'}\n'), 'original function boundary drift')
    return result


def compose(root=ROOT):
    raw = (HERE / 'inputs.json').read_bytes()
    require(len(raw) <= LIMIT, 'contract budget')
    c = json.loads(raw)
    require(c['schema'] == 1 and not any(c['execution'].values()), 'not an unexecuted source candidate')
    require(len(c['production_sources']) == 2 and len(c['experiment_sources']) == 2, 'input inventory')
    actual = {p: read(root, p, h) for p, h in c['production_sources'].items()}
    extra = {p: read(HERE, p, h) for p, h in c['experiment_sources'].items()}
    original = extract(actual[PROVIDERS])
    require(sha(original) == c['original_predicate_sha256'], 'original function hash')
    geometry = actual[GEOMETRY]
    # Preserve the full production module, including its seven original tests.
    # No model, extracted enum, replaced constructor, new field, or mutation API.
    generated = (b'mod geometry {\n' + geometry + b'\n}\n'
                 b'mod original {\nuse super::geometry::Batch;\n' + original + b'}\n'
                 b'mod candidate {\n' + extra['candidate.rs'] + b'\n}\n' + extra['fixture.rs'])
    require(len(generated) <= LIMIT, 'generated source budget')
    names = re.findall(rb'#\[test\]\s*fn\s+(\w+)\(', generated)
    require(len(names) == len(set(names)) == 14 and
            sorted(n.decode() for n in names) == sorted(c['expected_tests']), 'test inventory')
    require(b'#[ignore]' not in generated and b'#![' not in generated, 'test/module attributes')
    require(actual == {p: read(root, p, h) for p, h in c['production_sources'].items()}, 'changed production inputs')
    require(extra == {p: read(HERE, p, h) for p, h in c['experiment_sources'].items()}, 'changed fixture inputs')
    require(raw == (HERE / 'inputs.json').read_bytes(), 'changed contract')
    return generated, {'status': 'source_composed_only', 'generated_sha256': sha(generated),
                       'generated_bytes': len(generated), 'original_predicate_sha256': sha(original),
                       'production_module_preserved': True, 'prepared_tests': len(names),
                       'execution': c['execution'], 'performance_result': None}


if __name__ == '__main__':
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--mod-root', type=Path, default=ROOT)
    args = p.parse_args()
    _, result = compose(args.mod_root)
    print(json.dumps(result, sort_keys=True))
