"""The additive private-baseline proof uses real applied native sources."""
import importlib.util
from pathlib import Path
import sys
import unittest

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / 'tools'))
SPEC = importlib.util.spec_from_file_location('stack_fixture', ROOT / 'tools/prepare_stack_snapshot_fixture.py')
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class StackSnapshotFixture(unittest.TestCase):
    def passed(self):
        return '\n'.join('test boundary::tests::' + name + ' ... ok' for name in MODULE.REQUIRED_TESTS) + '\ntest result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s\n'

    def test_all_eight_actual_cases_required(self):
        MODULE.require_tests(self.passed())

    def test_missing_ignored_failed_or_filtered_test_rejects(self):
        complete = self.passed()
        for output in [complete.replace(' ... ok', ' ... ignored', 1),
                       complete.replace(' ... ok', ' ... FAILED', 1),
                       complete.replace('0 filtered out', '1 filtered out'),
                       complete.replace('8 passed', '0 passed'),
                       complete[complete.index('\n') + 1:]]:
            with self.assertRaises(ValueError):
                MODULE.require_tests(output)

    def test_actual_applied_items_and_single_api_identity_are_required(self):
        source = (ROOT / 'tools/prepare_stack_snapshot_fixture.py').read_text()
        for text in ('clean_official(official, source[\'horizon\'])',
                     'files(stage) != seal', 'guard.check(applied, official)',
                     "stage / 'crates/horizon-items'", "entries[0]['files']",
                     'path.is_relative_to(stage)', 'actual - allowed - LOCAL',
                     'files(stage) != evidence[\'stage_files_sha256\']',
                     'dependencies(output, official, stage, api, locked=True)'):
            self.assertIn(text, source)
        for forbidden in ('pub struct Wire', 'struct StackData', 'struct ItemStack',
                          '.write_bytes(reader', 'cfg('):
            self.assertNotIn(forbidden, source)

    def test_native_test_source_contains_no_reimplementation(self):
        source = (ROOT / 'tests/native/stack_snapshot.rs').read_text()
        for name in MODULE.REQUIRED_TESTS:
            self.assertIn('fn ' + name + '()', source)
        for forbidden in ('struct Wire', 'struct ItemStack', 'impl Wire', 'unsafe', 'Arc::ptr_eq'):
            self.assertNotIn(forbidden, source)
        self.assertIn('shared_baseline_graph(24, 1)', source)
        self.assertIn('encode_item_lossless(&item).is_none()', source)


if __name__ == '__main__':
    unittest.main()
