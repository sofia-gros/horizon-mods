"""Composition validator tests with synthetic plan inputs, not native execution."""
from pathlib import Path
import copy
import importlib.util
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location('cloud_plan', ROOT / 'tools/check_cloud_insertion_plan.py')
GUARD = importlib.util.module_from_spec(SPEC); SPEC.loader.exec_module(GUARD)


class CloudInsertionPlan(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.workspace = Path(self.temp.name)
        self.meta = GUARD.metadata()
        # Small opaque bytes test guard logic only; actual official hashes are
        # enforced by the normal metadata in the real complete-plan invocation.
        for name in self.meta['official_source_sha256']:
            path = self.workspace / name; path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text('native fixture input ' + name)
            self.meta['official_source_sha256'][name] = GUARD.digest(path.read_bytes())
        self.plan = {'schema': 6, 'inputs': dict(self.meta['official_source_sha256']),
            'injections': [], 'states': [], 'modules': []}
        for module in self.meta['modules']:
            entry = {key: module[key] for key in ('module_id', 'file', 'name')}
            entry.update(mod_id='pluto_native_creative',
                source=(ROOT / 'mixins/pluto_native_creative' / module['source_file']).read_text())
            if module['visibility'] == 'pub':
                entry['visibility'] = 'pub'
            self.plan['modules'].append(entry)

    def check(self, plan=None, meta=None):
        GUARD.check(self.plan if plan is None else plan, self.workspace,
                    self.meta if meta is None else meta)

    def test_exact_modules_and_omitted_default_visibility_are_accepted(self):
        self.check()
        for entry in self.plan['modules']:
            entry.setdefault('visibility', 'pub(crate)')
        self.check()

    def test_native_bytes_and_plan_input_drift_both_reject(self):
        for name in self.meta['official_source_sha256']:
            with self.subTest(name=name, kind='plan'):
                plan = copy.deepcopy(self.plan); plan['inputs'][name] = 'changed'
                with self.assertRaises(ValueError): self.check(plan)
            path = self.workspace / name; previous = path.read_bytes()
            path.write_bytes(previous + b' changed')
            with self.subTest(name=name, kind='native'):
                with self.assertRaises(ValueError): self.check()
            path.write_bytes(previous)

    def test_missing_duplicate_relocated_and_wrong_owner_modules_reject(self):
        for index in range(len(self.plan['modules'])):
            for change in ('missing', 'duplicate', 'relocate', 'owner', 'id', 'visibility', 'source'):
                with self.subTest(index=index, change=change):
                    plan = copy.deepcopy(self.plan); entry = plan['modules'][index]
                    if change == 'missing': plan['modules'].pop(index)
                    elif change == 'duplicate': plan['modules'].append(dict(entry))
                    elif change == 'relocate': entry['file'] = 'wrong.rs'
                    elif change == 'owner': entry['mod_id'] = 'other'
                    elif change == 'id': entry['module_id'] = 'other-id'
                    elif change == 'visibility': entry['visibility'] = 'private'
                    else: entry['source'] += '\n// changed'
                    with self.assertRaises(ValueError): self.check(plan)

    def test_even_local_helper_drift_rejects(self):
        for index in range(len(self.meta['modules'])):
            meta = copy.deepcopy(self.meta); meta['modules'][index]['source_sha256'] = 'changed'
            with self.assertRaises(ValueError): self.check(meta=meta)

    def test_hooks_states_constructors_and_private_modules_cannot_change_reserved_files(self):
        for file in self.meta['exclusive_files']:
            for section, entry in (
                ('injections', {'file': file, 'target': 'unrelated-name'}),
                ('states', {'file': file, 'target': 'unrelated-name'}),
                ('states', {'file': 'elsewhere.rs', 'constructors': [{'file': file}]}),
                ('modules', {'file': file, 'name': 'foreign', 'mod_id': 'other'}),
            ):
                with self.subTest(file=file, section=section):
                    plan = copy.deepcopy(self.plan); plan[section].append(entry)
                    with self.assertRaises(ValueError): self.check(plan)

    def test_protected_targets_reject_even_when_file_is_misreported(self):
        for prefix in self.meta['protected_target_prefixes']:
            for section in ('injections', 'states'):
                plan = copy.deepcopy(self.plan)
                plan[section].append({'file': 'elsewhere.rs', 'target': prefix + 'function'})
                with self.assertRaises(ValueError): self.check(plan)

    def test_other_game_hooks_are_not_blanket_blocked_but_game_state_is(self):
        file = 'crates/horizon-client/src/game.rs'
        self.plan['injections'].append({'file': file, 'target': "horizon_client::game::Game<'_>::use_tool"})
        self.check()
        for entry in ({'file': file}, {'file': 'elsewhere.rs', 'constructors': [{'file': file}]}):
            plan = copy.deepcopy(self.plan); plan['states'].append(entry)
            with self.assertRaises(ValueError): self.check(plan)

    def test_unknown_schema_rejects(self):
        self.plan['schema'] = 7
        with self.assertRaises(ValueError): self.check()


if __name__ == '__main__':
    unittest.main()
