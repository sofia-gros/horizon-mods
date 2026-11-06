"""Read-only source guards; never builds or executes the native overlay fixture."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import unittest

HERE = Path(__file__).resolve().parent
INPUTS = json.loads((HERE / 'inputs.json').read_text())
HOST = None

class OverlaySource(unittest.TestCase):
    def test_frozen_fixture_and_eight_names(self):
        for path, digest in INPUTS['fixture_inputs'].items():
            self.assertEqual(hashlib.sha256((HERE/path).read_bytes()).hexdigest(), digest)
        names = ['tests::'+n for n in re.findall(r'#\[test\]\s+fn (\w+)\(', (HERE/'fixture.rs').read_text())]
        self.assertEqual(names, INPUTS['expected_test_names'])
        self.assertEqual(len(names), 8)
        self.assertEqual(len(set(names)), 8)

    def test_actual_public_owners_without_substitute_pipeline(self):
        text = (HERE/'fixture.rs').read_text()
        for token in ['horizon_client_lib::mod_content::{self, ContentMod}', 'Package::write_files(', 'Package::read(', 'Content::read(', 'mod_content::build(', 'Language::load(&[&out]']:
            self.assertIn(token, text)
        for token in ['struct Language', 'struct Overlay', 'impl ResourceSource', 'lang::set(', '#[ignore]', 'Command::new', 'serde_json::']:
            self.assertNotIn(token, text)

    def test_owned_outputs_and_failure_phase_are_explicit(self):
        text = (HERE/'fixture.rs').read_text()
        for token in ['fn output(&self) -> PathBuf { self.0.join("owned-output") }', 'fs::create_dir(&p)', 'fs::remove_dir_all(&self.0)', 'source_bytes(&base), before', 'before_owned_output_recreation', 'after_owned_output_recreation_without_rollback_claim', 'whole-file replacement, not key retention']:
            self.assertIn(token, text)
        self.assertNotIn('std::env::args', text)
        self.assertNotIn('std::env::var', text)

    def test_dependency_requires_actual_native_client_library(self):
        text = (HERE/'Cargo.toml.in').read_text()
        self.assertIn('horizon-client-lib = { path = "@OFFICIAL@/crates/horizon-client-lib", default-features = false }', text)
        self.assertIn('path = "fixture.rs"', text)
        self.assertNotIn('[patch.', text)

    def test_exact_approved_official_source(self):
        env = {**os.environ, 'GIT_OPTIONAL_LOCKS':'0'}
        def git(*args): return subprocess.check_output(['git','-C',str(HOST),*args], env=env)
        self.assertEqual(git('remote','get-url','origin').decode().strip(), INPUTS['official_url'])
        head = git('rev-parse','HEAD').decode().strip()
        self.assertEqual(head, INPUTS['horizon'])
        for item in INPUTS['official_inputs']:
            data = (HOST/item['path']).read_bytes()
            self.assertEqual(data, git('show',head+':'+item['path']), item['path'])
            self.assertEqual(hashlib.sha256(data).hexdigest(), item['sha256'], item['path'])

    def test_no_execution_or_locale_priority_claim(self):
        self.assertFalse(INPUTS['execution']['run_now'])
        self.assertEqual(len(INPUTS['excluded']), 8)
        text = (HERE/'README.md').read_text()
        for token in ['uncompiled and unrun', 'No runner is dispatched', 'does **not** run selected/server pack', 'not changing', 'atomic rollback', 'same extra namespace']:
            self.assertIn(token, text)

if __name__ == '__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--host', type=Path, required=True)
    HOST=parser.parse_args().host.resolve()
    unittest.main(argv=[__file__], verbosity=2)
