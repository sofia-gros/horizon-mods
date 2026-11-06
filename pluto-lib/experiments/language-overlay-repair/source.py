#!/usr/bin/env python3
"""Excluded exact-source candidate composer. Returns bytes; no apply or tools."""
import argparse
import hashlib
import json
from pathlib import Path
import re

HERE = Path(__file__).resolve().parent
HORIZON = "f4dcf8a2fb4a68b2d28422f4d34d2be2970e26bd"
TARGET = "crates/horizon-client/src/mod_content.rs"
MAX_SOURCE = 2 * 1024 * 1024
WRITER_OLD = b'                overlay.write(file, &bytes)?;\n'
WRITER_NEW = b'''                {
                    let pluto_language_bytes = pluto_language_overlay::prepare(base, file, &bytes);
                    overlay.write(file, &pluto_language_bytes)?;
                }
'''
STAMP_OLD = b'    let mut parts = vec!["horizon mod content 1".to_owned()];\n'
STAMP_NEW = STAMP_OLD + b'    parts.push(pluto_language_overlay::POLICY_ID.to_owned());\n'
MODULE_BEGIN = b'\n// BEGIN EXCLUDED PLUTO LANGUAGE OVERLAY CANDIDATE\nmod pluto_language_overlay {\n'
MODULE_END = b'\n}\n// END EXCLUDED PLUTO LANGUAGE OVERLAY CANDIDATE\n'


def sha(value):
    return hashlib.sha256(value).hexdigest()


def require(ok, message):
    if not ok:
        raise ValueError(message)


def read(root, relative, digest):
    root, relative = Path(root).absolute(), Path(relative)
    require(not relative.is_absolute() and '..' not in relative.parts, 'unsafe input path')
    path = root / relative
    # Quiescent source contract; not a defense against concurrent file swaps.
    require(not any(p.is_symlink() for p in (path, *path.parents)), 'symlink input component')
    require(path.is_file() and path.stat().st_size <= MAX_SOURCE, 'invalid input file')
    with path.open('rb') as handle:
        raw = handle.read(MAX_SOURCE + 1)
    require(len(raw) <= MAX_SOURCE and sha(raw) == digest, 'input hash mismatch: ' + str(relative))
    return raw


def candidate(native, helper):
    require(native.count(WRITER_OLD) == 1, 'native admitted-Mod writer changed')
    require(native.count(STAMP_OLD) == 1, 'native stamp domain changed')
    require(b'pluto_language_overlay' not in native, 'candidate already present')
    require(MODULE_BEGIN not in helper and MODULE_END not in helper, 'reserved module marker')
    require(0 < len(helper) < 65536, 'helper size limit')
    result = native.replace(WRITER_OLD, WRITER_NEW, 1).replace(STAMP_OLD, STAMP_NEW, 1)
    result += MODULE_BEGIN + helper + MODULE_END
    require(inverse(result, helper) == native, 'native inverse mismatch')
    return result


def inverse(value, helper):
    tail = MODULE_BEGIN + helper + MODULE_END
    require(value.endswith(tail) and value.count(MODULE_BEGIN) == 1, 'candidate module boundary changed')
    body = value[:-len(tail)]
    require(body.count(WRITER_NEW) == 1 and body.count(STAMP_NEW) == 1, 'candidate seams changed')
    return body.replace(WRITER_NEW, WRITER_OLD, 1).replace(STAMP_NEW, STAMP_OLD, 1)


def compose(selected_root):
    raw_contract = (HERE / 'inputs.json').read_bytes()
    require(len(raw_contract) < 32768, 'contract size limit')
    contract = json.loads(raw_contract)
    require(contract['schema'] == 1 and contract['horizon'] == HORIZON and
            contract['official_url'] == 'https://github.com/horizon-rust/horizon.git', 'wrong official contract')
    require(contract['status'] == 'excluded source candidate; no native execution or activation', 'execution claim changed')
    sources = {p: read(selected_root, p, digest) for p, digest in contract['official_sources'].items()}
    admission = contract['admission_contract']
    body = sources[admission['path']]
    start, end = admission['start'].encode(), admission['end'].encode()
    require(body.count(start) == body.count(end) == 1, 'admission bounds changed')
    at, until = body.index(start), body.index(end)
    require(at < until and sha(body[at:until]) == admission['sha256'], 'admission namespace contract changed')
    helper = read(HERE, 'merge.rs', contract['helper_sha256'])
    generated = candidate(sources[TARGET], helper)
    names = re.findall(rb'#\[test\]\s*fn\s+(\w+)\(', helper)
    require([n.decode() for n in names] == contract['prepared_tests'] and len(set(names)) == 14,
            'helper Rust inventory changed')
    require(b'#[ignore]' not in helper, 'ignored helper test')
    require(sources == {p: read(selected_root, p, digest) for p, digest in contract['official_sources'].items()},
            'official source changed during composition')
    require(helper == read(HERE, 'merge.rs', contract['helper_sha256']) and
            raw_contract == (HERE / 'inputs.json').read_bytes(), 'candidate inputs changed')
    return generated, {'status': 'source_composed_only', 'horizon': HORIZON,
                       'source_sha256': sha(sources[TARGET]), 'helper_sha256': sha(helper),
                       'candidate_sha256': sha(generated), 'candidate_bytes': len(generated),
                       'prepared_rust_tests': len(names), 'native_executed': False,
                       'mixins_placement_verified': False, 'production_activated': False}


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--selected-source', type=Path, required=True)
    args = parser.parse_args()
    _, result = compose(args.selected_source)
    print(json.dumps(result, sort_keys=True))
