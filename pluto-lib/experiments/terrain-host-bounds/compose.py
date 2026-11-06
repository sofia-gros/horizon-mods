#!/usr/bin/env python3
"""Excluded one-call host comparison source. No plan, installation or execution."""
import argparse
import hashlib
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
LIMIT = 128 * 1024
HOST = 'pluto-lib/mixins/pluto_native_creative/terrain-cell-host.rs'
CANDIDATE = 'pluto-lib/experiments/terrain-local-bounds/candidate.rs'
BEFORE = b'api::cell_local(batch)'
AFTER = b'local_bounds_candidate::cell_local(batch)'
MODULE = b'\n// EXCLUDED comparison module; not an installed optimization.\nmod local_bounds_candidate {\n'
MODULE_NAME = 'pluto_terrain_cell_host_bounds_probe'


def require(value, message):
    if not value: raise ValueError(message)


def sha(raw): return hashlib.sha256(raw).hexdigest()


def read(root, relative, digest):
    parts = relative.split('/')
    require(parts and all(p not in ('', '.', '..') for p in parts) and '\\' not in relative,
            'unsafe relative path')
    path = (Path(root) / relative).absolute()
    require(not any(p.is_symlink() for p in (path, *path.parents)), 'symlink input component')
    require(path.is_file() and path.stat().st_size <= LIMIT, 'invalid input file')
    with path.open('rb') as handle: raw = handle.read(LIMIT + 1)
    require(len(raw) <= LIMIT and sha(raw) == digest, 'source hash mismatch: '+relative)
    return raw


def transform(host, candidate):
    require(len(host) <= LIMIT and len(candidate) <= LIMIT, 'source budget')
    require(host.count(BEFORE) == 1 and AFTER not in host and b'local_bounds_candidate' not in host,
            'ambiguous original validation call')
    require(candidate.count(b'pub(super) fn cell_local(') == 1 and
            b'&super::geometry::Batch' in candidate and candidate.endswith(b'    })\n}\n'),
            'candidate signature/body boundary')
    require(b'#![' not in candidate and b'\nmod ' not in candidate, 'unsupported candidate module shape')
    generated = host.replace(BEFORE, AFTER, 1) + MODULE + candidate + b'}\n'
    require(len(generated) <= LIMIT, 'generated source budget')
    return generated


def inverse(generated, candidate, expected_host_sha256):
    suffix = MODULE + candidate + b'}\n'
    require(generated.endswith(suffix), 'missing exact internal candidate module')
    body = generated[:-len(suffix)]
    require(body.count(AFTER) == 1 and BEFORE not in body, 'ambiguous transformed call')
    original = body.replace(AFTER, BEFORE, 1)
    require(sha(original) == expected_host_sha256, 'full original host inverse mismatch')
    return original


def compose(root=ROOT):
    raw = (HERE/'inputs.json').read_bytes()
    require(len(raw) <= LIMIT, 'contract budget')
    c = json.loads(raw)
    require(c['schema'] == 1 and not any(c['execution'].values()), 'not an unexecuted source candidate')
    require(set(c['source_sha256']) == {HOST, CANDIDATE,
        'pluto-lib/mixins/pluto_native_creative/terrain-cell-geometry.rs',
        'pluto-lib/mixins/pluto_native_creative/terrain-cell-providers.rs',
        'pluto-lib/experiments/terrain-local-bounds/inputs.json'}, 'source inventory')
    inputs = {p:read(root,p,h) for p,h in c['source_sha256'].items()}
    generated = transform(inputs[HOST],inputs[CANDIDATE])
    require(inverse(generated,inputs[CANDIDATE],c['source_sha256'][HOST]) == inputs[HOST], 'inverse drift')
    require(sha(generated) == c['generated_sha256'], 'generated source drift')
    require(inputs == {p:read(root,p,h) for p,h in c['source_sha256'].items()}, 'source changed while composing')
    require(raw == (HERE/'inputs.json').read_bytes(), 'contract changed while composing')
    return generated, {'status':'excluded_source_only','module_name':MODULE_NAME,
        'original_host_sha256':c['source_sha256'][HOST], 'candidate_sha256':c['source_sha256'][CANDIDATE],
        'generated_sha256':sha(generated), 'generated_bytes':len(generated),
        'changed_call_sites':1, 'original_host_fully_reconstructed':True,
        'manifest_created':False, 'performance_result':None, 'execution':c['execution']}


if __name__ == '__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--mod-root',type=Path,default=ROOT)
    _,receipt=compose(parser.parse_args().mod_root)
    print(json.dumps(receipt,sort_keys=True))
