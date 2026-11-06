"""Source-only guards. This neither prepares nor compiles/runs the Rust fixture."""
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


class PackageSource(unittest.TestCase):
    def test_fixture_exact_bytes_and_eight_named_cases(self):
        for path, expected in INPUTS['fixture_inputs'].items():
            self.assertEqual(hashlib.sha256((HERE / path).read_bytes()).hexdigest(), expected)
        source = (HERE / 'fixture.rs').read_text()
        names = ['tests::' + n for n in re.findall(r'#\[test\]\s+fn (\w+)\(', source)]
        self.assertEqual(names, INPUTS['expected_test_names'])
        self.assertEqual(len(names), 8)
        self.assertEqual(len(set(names)), 8)

    def test_actual_native_owners_and_no_substitute_loader(self):
        source = (HERE / 'fixture.rs').read_text()
        for text in ['Package::write_files(', 'Package::read(', 'Content::read(',
                     'self.0.file(path).expect(', 'Language::load(', 'lang::available(']:
            self.assertIn(text, source)
        for text in ['struct Language', 'struct Content', 'struct Package', 'lang::set(',
                     'serde_json', '#[ignore]', 'Command::new', 'mod_content::build']:
            self.assertNotIn(text, source)
        view = source.split('impl ResourceSource for Admitted', 1)[1].split('\n    #[test]', 1)[0]
        self.assertIn('binary_search_by', view)
        self.assertNotIn('.ok()', view)
        self.assertIn('admitted resource must remain readable', view)

    def test_real_dependency_edges_are_explicit_and_small(self):
        template = (HERE / 'Cargo.toml.in').read_text()
        self.assertIn('horizon-client-core = { path = "@OFFICIAL@/crates/horizon-client-core" }', template)
        self.assertIn('horizon-mods = { path = "@OFFICIAL@/crates/horizon-mods", default-features = false }', template)
        self.assertNotIn('horizon-client =', template)
        self.assertNotIn('horizon-client-render =', template)
        self.assertIn('path = "fixture.rs"', template)

    def test_official_revision_and_exact_input_bytes(self):
        env = {**os.environ, 'GIT_OPTIONAL_LOCKS': '0'}
        head = subprocess.check_output(['git', '-C', str(HOST), 'rev-parse', 'HEAD'], env=env, text=True).strip()
        self.assertEqual(head, INPUTS['horizon'])
        for item in INPUTS['official_inputs']:
            actual = (HOST / item['path']).read_bytes()
            committed = subprocess.check_output(['git', '-C', str(HOST), 'show', head + ':' + item['path']], env=env)
            self.assertEqual(actual, committed, item['path'])
            self.assertEqual(hashlib.sha256(actual).hexdigest(), item['sha256'], item['path'])

    def test_execution_and_production_boundaries_stay_explicit(self):
        self.assertFalse(INPUTS['execution']['run_now'])
        self.assertIn('uncompiled', INPUTS['status'])
        self.assertEqual(len(INPUTS['excluded']), 6)
        readme = (HERE / 'README.md').read_text()
        for text in ['test-only', 'uncompiled and unrun', 'not a newly', 'No runner is dispatched']:
            self.assertIn(text, readme)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--host', type=Path, required=True)
    args = parser.parse_args()
    HOST = args.host.resolve()
    unittest.main(argv=[__file__])
