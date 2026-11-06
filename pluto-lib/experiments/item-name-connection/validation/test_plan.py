"""Adversarial guard tests using synthetic plan records, not an accepted plan."""
import copy
import importlib.util
from pathlib import Path
import unittest

HERE = Path(__file__).resolve().parent
SPEC = importlib.util.spec_from_file_location('item_name_plan', HERE / 'check_plan.py')
guard = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(guard)

class PlanTests(unittest.TestCase):
    def setUp(self):
        self.meta = guard.metadata()
        self.candidate = HERE.parent / 'candidate'
        self.plan = {'schema': 6, 'injections': [], 'modules': [], 'inputs': dict(self.meta['native_sha256'])}
        for entry in self.meta['injections']:
            hook = copy.deepcopy(entry['expected'])
            hook['helper_source'] = (self.candidate / entry['helper']).read_text()
            self.plan['injections'].append(hook)
        for entry in self.meta['modules']:
            self.plan['modules'].append({
                'mod_id': guard.OWNER, 'mod_version': '0.1.0', 'module_id': entry['module_id'],
                'file': entry['file'], 'name': entry['name'],
                'source': (self.candidate / entry['helper']).read_text(),
                # Structural synthetic value. This is NOT an official plan fingerprint.
                'fingerprint': '0' * 64,
            })

    def check(self):
        guard.check_structure(self.plan, self.meta, self.candidate)

    def test_exact_synthetic_shape_is_accepted(self):
        self.check()

    def test_unknown_schema_is_rejected(self):
        self.plan['schema'] = 7
        with self.assertRaises(ValueError): self.check()

    def test_missing_or_duplicate_head_is_rejected(self):
        original = copy.deepcopy(self.plan)
        self.plan['injections'].pop()
        with self.assertRaises(ValueError): self.check()
        self.plan = original
        self.plan['injections'].append(copy.deepcopy(self.plan['injections'][-1]))
        with self.assertRaises(ValueError): self.check()

    def test_other_owner_is_rejected(self):
        self.plan['injections'][-1]['mod_id'] = 'other'
        with self.assertRaises(ValueError): self.check()

    def test_helper_or_priority_drift_is_rejected(self):
        original = copy.deepcopy(self.plan)
        self.plan['injections'][-1]['helper_source'] += '\n'
        with self.assertRaises(ValueError): self.check()
        self.plan = original
        self.plan['injections'][-1]['priority'] += 1
        with self.assertRaises(ValueError): self.check()

    def test_fingerprint_selector_or_capture_drift_is_rejected(self):
        original = copy.deepcopy(self.plan)
        for key, value in [('function_fingerprint', 'a' * 64), ('selector', 'other()'), ('captures', [{}])]:
            self.plan = copy.deepcopy(original)
            self.plan['injections'][0][key] = value
            with self.assertRaises(ValueError): self.check()

    def test_unrelated_guest_hook_is_rejected(self):
        self.plan['injections'].append({'file': guard.GUEST, 'target': 'horizon_client_lib::lan::guest::other'})
        with self.assertRaises(ValueError): self.check()

    def test_component_producer_hook_is_rejected(self):
        self.plan['injections'].append({'file': guard.SYNC, 'target': 'horizon_client_lib::item_sync::components_tag'})
        with self.assertRaises(ValueError): self.check()

    def test_extra_guest_module_is_rejected(self):
        self.plan['modules'].append({'file': guard.GUEST, 'name': 'other'})
        with self.assertRaises(ValueError): self.check()

    def test_gate_module_source_or_visibility_drift_is_rejected(self):
        original = copy.deepcopy(self.plan)
        self.plan['modules'][-1]['source'] += '\n'
        with self.assertRaises(ValueError): self.check()
        self.plan = original
        self.plan['modules'][-1]['visibility'] = 'pub'
        with self.assertRaises(ValueError): self.check()

    def test_bridge_state_or_constructor_in_protected_file_is_rejected(self):
        original = copy.deepcopy(self.plan)
        for key, value in [('bridges', {'file': guard.GUEST}), ('states', {'file': guard.SYNC}),
                           ('states', {'file': 'unrelated.rs', 'constructors': [{'file': guard.GUEST}]})]:
            self.plan = copy.deepcopy(original)
            self.plan[key] = [value]
            with self.assertRaises(ValueError): self.check()

    def test_post_build_mutation_is_rejected(self):
        self.plan['post_build'] = [{'command': 'mutate'}]
        with self.assertRaises(ValueError): self.check()

if __name__ == '__main__':
    unittest.main()
