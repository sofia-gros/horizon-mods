"""Read-only exact reversal checks; not Rust or native Font execution."""
import hashlib
import json
from pathlib import Path
import re
import subprocess
import unittest

ROOT = Path(__file__).resolve().parents[3]
HERE = Path(__file__).resolve().parent
DATA = json.loads((HERE / 'inputs.json').read_text())

def read(path):
    value = ROOT / path
    if value.is_file():
        return value.read_bytes()
    # Sparse tracked sources are read from this MOD checkout, never a host tree.
    return subprocess.check_output(['git', '-C', str(ROOT), 'show', 'HEAD:' + path])
def sha(data): return hashlib.sha256(data).hexdigest()

class Source(unittest.TestCase):
    def test_complete_generator_inverse(self):
        path = 'tconstruct/mixins/tconstruct/source-materials-book.rs'
        new = read(path).decode()
        self.assertEqual(new.count(DATA['replacement']), 1)
        self.assertEqual(sha(new.replace(DATA['replacement'], DATA['old_body']).encode()), DATA['old_sha256'][path])
    def test_support_is_only_additive_and_inverse_exact(self):
        path = 'tconstruct/mixins/tconstruct/source-materials-book-text.rs'
        new = read(path).decode()
        self.assertTrue(new.endswith(DATA['addition']))
        self.assertEqual(sha(new[:-len(DATA['addition'])].encode()), DATA['old_sha256'][path])
    def test_generation_order_and_single_native_fallback(self):
        source = DATA['addition']
        paginate, split = source.split('fn generation_lines(', 1)
        self.assertEqual(paginate.count('resolve_legacy()'), 1)
        self.assertEqual(paginate.count('generation_lines('), 1)
        self.assertIn('chunks(rows.max(1))', paginate)
        self.assertEqual(split.count('font.split('), 1)
        self.assertEqual(split.count('layout::split_raw('), 1)
        self.assertLess(split.index('from_native('), split.index('layout::split_raw('))
        for token in ['advance(', 'width_text(', 'resolve_legacy(', 'Game<', 'send_packet(', 'open_book(']:
            self.assertNotIn(token, split)
    def test_unchanged_wire_hooks_native_helpers_and_prior_fixture(self):
        for path, digest in DATA['unchanged_sha256'].items():
            self.assertEqual(sha(read(path)), digest, path)
    def test_prepared_fixture_has_six_cases_and_original_oracle(self):
        source = (HERE / 'native.rs').read_text()
        self.assertEqual(sha(source.encode()), DATA['fixture_sha256'])
        self.assertEqual(source.count('#[test]'), 6)
        self.assertIn('include!("../guide-text-migration/native.rs");', source)
        oracle = source.split('fn old_paginate(', 1)[1].split('fn compare_pages(', 1)[0]
        self.assertNotIn('consumer::', oracle)
        self.assertNotIn('pluto_', oracle)
        original = DATA['old_body'].replace('game.gui.atlas.font', 'font')
        self.assertIn(original.strip(), '\n'.join(line[4:] if line.startswith('    ') else line for line in oracle.splitlines()))
        self.assertIn('emitted.len(), if full { 1 } else { 2 }', source)
    def test_exact_changed_source_hashes(self):
        for path, digest in DATA['new_sha256'].items():
            self.assertEqual(sha(read(path)), digest, path)

if __name__ == '__main__': unittest.main()
