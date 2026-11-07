"""The borrowed private-field seam must fail closed on source/composition drift."""
import copy
import importlib.util
from pathlib import Path
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location('wire_guard', ROOT / 'tools/check_wire_snapshot_plan.py')
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class WireSnapshotPlan(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix='wire-source-guard-')
        self.addCleanup(self.temp.cleanup)
        self.host = Path(self.temp.name)
        self.meta = MODULE.metadata()
        source = ('// Test-only source fixture\n' + self.meta['declaration'] + '\n').encode()
        path = self.host / self.meta['file']; path.parent.mkdir(parents=True); path.write_bytes(source)
        self.meta['file_sha256'] = MODULE.digest(source)
        self.plan = {'modules': [dict(mod_id='pluto_native_creative', module_id='pluto_native_creative-' + id,
            file=file, name=name, visibility='pub', source=(ROOT / 'mixins/pluto_native_creative' / source).read_text())
            for id, (file, name, source) in MODULE.MODULES.items()], 'injections': [], 'states': []}

    def test_expected_read_only_modules_pass(self):
        MODULE.check(self.plan, self.host, self.meta)

    def test_host_source_and_declaration_drift_fail(self):
        path = self.host / self.meta['file']; path.write_bytes(path.read_bytes() + b'// drift\n')
        with self.assertRaisesRegex(ValueError, 'Official private Wire'):
            MODULE.check(self.plan, self.host, self.meta)
        self.meta['file_sha256'] = MODULE.digest(path.read_bytes())
        self.meta['declaration'] = 'pub struct DifferentWire;'
        with self.assertRaisesRegex(ValueError, 'Official private Wire'):
            MODULE.check(self.plan, self.host, self.meta)

    def test_reader_bytes_are_independently_reviewed(self):
        self.meta['reader_sha256'] = 'wrong'
        with self.assertRaisesRegex(ValueError, 'Read-only Wire adapter'):
            MODULE.check(self.plan, self.host, self.meta)

    def test_missing_duplicate_retargeted_or_changed_modules_fail(self):
        for index in range(3):
            changes = []
            missing = copy.deepcopy(self.plan); missing['modules'].pop(index); changes.append(missing)
            duplicate = copy.deepcopy(self.plan); duplicate['modules'].append(duplicate['modules'][index]); changes.append(duplicate)
            for key, value in [('source', '// different'), ('visibility', 'pub(crate)'), ('file', 'other.rs'), ('mod_id', 'other')]:
                changed = copy.deepcopy(self.plan); changed['modules'][index][key] = value; changes.append(changed)
            for changed in changes:
                with self.assertRaises(ValueError):
                    MODULE.check(changed, self.host, self.meta)

    def test_additional_item_data_hooks_or_state_require_review(self):
        for section in ('injections', 'states'):
            for change in [{'file': self.meta['file']}, {'target': 'horizon_items::item_data::Wire::new'}]:
                plan = copy.deepcopy(self.plan); plan[section].append(change)
                with self.assertRaisesRegex(ValueError, 'Additional item_data'):
                    MODULE.check(plan, self.host, self.meta)

    def test_exact_comparison_reuses_existing_policy_and_completes_before_memoizing(self):
        source = (ROOT / 'mixins/pluto_native_creative/stack-snapshot.rs').read_text()
        self.assertIn('pluto_stack_storage::same_item_snapshot(before, after)', source)
        self.assertIn('pluto_stack_storage::nested_stacks(before)', source)
        self.assertIn('Work::Complete(pair) => {', source)
        self.assertIn('completed.insert(pair);', source)
        self.assertLess(source.index('pending.push(Work::Complete(pair))'), source.index('pending.push(Work::Compare(before, after))'))
        for forbidden in ('Arc::ptr_eq', 'unsafe', 'same_item_same_components', '.wire.0'):
            self.assertNotIn(forbidden, source)


if __name__ == '__main__':
    unittest.main()
