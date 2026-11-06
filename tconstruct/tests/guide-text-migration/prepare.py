#!/usr/bin/env python3
"""Bounded actual Font/Text consumer fixture; no Game, screen, action or packet."""
import argparse
import hashlib
import importlib.util
import json
from pathlib import Path
import shutil

ROOT = Path(__file__).resolve().parents[3]
SPEC = importlib.util.spec_from_file_location('pluto_text_fixture', ROOT / 'pluto-lib/tools/prepare_guide_text_fixture.py')
BASE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(BASE)
CONSUMER = 'tconstruct/mixins/tconstruct/source-materials-book-text.rs'
TEST = 'tconstruct/tests/guide-text-migration/native.rs'

def sha(data): return hashlib.sha256(data).hexdigest()
def verify(output):
    evidence = BASE.verify(output)
    for path, digest in evidence['consumer_sources_sha256'].items():
        if sha((ROOT / path).read_bytes()) != digest:
            raise ValueError('Production/fixture source drift: ' + path)
    for local, origin in [('consumer.rs', CONSUMER), ('consumer_tests.rs', TEST),
                          ('guide-layout.rs', 'pluto-lib/mixins/pluto_native_creative/guide-layout.rs'),
                          ('guide-text.rs', 'pluto-lib/mixins/pluto_native_creative/guide-text.rs')]:
        if (output / local).read_bytes() != (ROOT / origin).read_bytes():
            raise ValueError('Fixture is not exact production source: ' + local)
    return evidence

def prepare(host, output):
    BASE.prepare(host, output, include_layout=True)
    shutil.copyfile(ROOT / CONSUMER, output / 'consumer.rs')
    shutil.copyfile(ROOT / TEST, output / 'consumer_tests.rs')
    with (output / 'lib.rs').open('a') as file:
        file.write('extern crate self as horizon_client;\npub use guide_layout as pluto_guide_layout;\n#[path="consumer.rs"] mod consumer;\n#[cfg(test)] #[path="consumer_tests.rs"] mod consumer_tests;\n')
    evidence = json.loads((output / 'fixture-sources.json').read_text())
    evidence['consumer_sources_sha256'] = {p: sha((ROOT / p).read_bytes()) for p in [CONSUMER, TEST]}
    evidence['files_sha256'] = {str(p.relative_to(output)): sha(p.read_bytes()) for p in output.rglob('*')
                              if p.is_file() and p.name != 'fixture-sources.json'}
    evidence['scope'] += '; exact consumer delegation and native fallback against frozen traversal oracle; no Game/type/apply/UI proof.'
    (output / 'fixture-sources.json').write_text(json.dumps(evidence, indent=2) + '\n')
    verify(output)

if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--horizon-source', type=Path)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--verify', action='store_true')
    args = parser.parse_args()
    if args.verify: verify(args.output.resolve())
    else: prepare(args.horizon_source.resolve(), args.output.resolve())
