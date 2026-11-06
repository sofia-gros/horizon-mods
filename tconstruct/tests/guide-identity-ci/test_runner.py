import contextlib
import importlib.util
import io
import json
from pathlib import Path
from types import SimpleNamespace
import unittest
from unittest import mock

HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location('identity_ci', HERE/'run.py')
m = importlib.util.module_from_spec(spec); spec.loader.exec_module(m)

class Checks(unittest.TestCase):
    def test_fixed_production_inputs(self):
        c, f = m.inputs()
        self.assertEqual(c['fixture_commit'], 'd9f8e6ba50fe7b1d9a3fc457bafa1650d3da8bd5')
        self.assertEqual(len(f['source_sha256']), 3)
        self.assertEqual(len(f['total_test_inventory']), 21)
        self.assertEqual(sum(g['count'] for g in f['groups']), 10)

    def test_exact_public_push_identity(self):
        e = {'GITHUB_ACTIONS':'true', 'GITHUB_REPOSITORY':m.REPOSITORY, 'GITHUB_EVENT_NAME':'push',
             'GITHUB_REF':'refs/heads/'+m.BRANCH, 'GITHUB_SHA':'abc'}
        event = {'repository':{'private':False, 'full_name':m.REPOSITORY}}
        m.validate_event(e, event, 'abc')
        for key in e:
            bad = dict(e); bad[key] = 'wrong'
            with self.assertRaises(ValueError): m.validate_event(bad, event, 'abc')
        for private in [True, None, 0]:
            with self.assertRaises(ValueError): m.validate_event(e, {'repository':{'private':private, 'full_name':m.REPOSITORY}}, 'abc')

    def test_inventory_requires_all_distinct_names(self):
        names = m.inputs()[1]['total_test_inventory']
        good = '\n'.join(n+': test' for n in names)+'\n21 tests, 0 benchmarks\n'
        m.validate_inventory(good, names)
        for bad in [good.replace(names[0], names[1]), good.replace('21 tests', '20 tests'), good+good, '']:
            with self.assertRaises(ValueError): m.validate_inventory(bad, names)

    def test_two_filtered_groups_require_every_success(self):
        for group in m.inputs()[1]['groups']:
            good = '\n'.join('test '+n+' ... ok' for n in group['names'])+'\ntest result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 16 filtered out; finished in 0.0s\n'
            m.validate_tests(good, group)
            for bad in [good.replace('16 filtered', '15 filtered'), good.replace('5 passed', '0 passed'), good.replace(' ... ok', ' ... ignored', 1), good.replace(group['names'][0], group['names'][1]), good+good, '']:
                with self.assertRaises(ValueError): m.validate_tests(bad, group)

    def test_python_checks_cannot_be_skipped(self):
        good = ''.join(f'test_{n} (Checks) ... ok\n' for n in range(m.RUNNER_TESTS))+f'Ran {m.RUNNER_TESTS} tests in 0.1s\nOK\n'
        m.validate_python(good)
        for bad in ['', good.replace(' ... ok', ' ... skipped', 1), good+good, good.replace('OK', 'FAILED')]:
            with self.assertRaises(ValueError): m.validate_python(bad)

    def test_disk_floor_includes_toolchain_and_root(self):
        for low in ['/', '/rustup', '/cargo', '/work', str(m.ROOT)]:
            with mock.patch.object(m, 'existing', side_effect=Path), mock.patch.object(m.shutil, 'disk_usage', side_effect=lambda p: SimpleNamespace(free=m.FLOOR-1 if str(p)==low else m.FLOOR+3*1024**3)):
                with self.assertRaises(RuntimeError): m.Runner(Path('/work'), {'RUSTUP_HOME':'/rustup', 'CARGO_HOME':'/cargo'}).guard()
        with mock.patch.object(m.shutil, 'disk_usage', return_value=SimpleNamespace(free=m.FLOOR+1024**3)):
            with self.assertRaises(RuntimeError): m.Runner(Path('/tmp'), {}).guard(reserve=2*1024**3)

    def test_group_cleanup_is_bounded(self):
        with mock.patch.object(m.os, 'killpg', side_effect=ProcessLookupError): m.ensure_group_gone(123, 'done')
        with mock.patch.object(m.os, 'killpg'), mock.patch.object(m.time, 'monotonic', side_effect=[0, 2]):
            with self.assertRaises(RuntimeError): m.ensure_group_gone(123, 'live')
        text = (HERE/'run.py').read_text()
        self.assertIn('proc.wait(timeout=2)', text)
        self.assertIn('ensure_group_gone(proc.pid, name)', text)

    def test_output_is_exclusive_and_compile_is_single(self):
        text = (HERE/'run.py').read_text()
        self.assertIn('work.mkdir()', text)
        self.assertNotIn('exist_ok=True', text)
        self.assertEqual(text.count("runner.run('compile',"), 1)
        self.assertIn("'-C', 'debuginfo=0'", text)
        self.assertIn("binary_seal(binary) != compiled", text)
        self.assertIn("seal(ROOT) != before", text)

    def test_dirty_checkout_rejected(self):
        with mock.patch.object(m, 'git', return_value='!! .cargo/config.toml'):
            with self.assertRaises(ValueError): m.clean()

    def test_failed_or_missing_gates_cannot_emit_true(self):
        def incomplete(r): r['verified'] = True
        for side in [incomplete, RuntimeError('compile failed')]:
            with mock.patch.object(m, 'execute', side_effect=side), mock.patch('builtins.open', mock.mock_open()), contextlib.redirect_stdout(io.StringIO()) as out:
                with self.assertRaises(SystemExit): m.main()
            receipt = json.loads(out.getvalue().split('FINAL_RECEIPT ')[1])
            self.assertFalse(receipt['passed']); self.assertTrue(receipt['error'])

    def test_summary_failure_revokes_success_receipt(self):
        def complete(r): r.update(verified=True, gates={n:True for n in m.REQUIRED})
        with mock.patch.object(m, 'execute', side_effect=complete), mock.patch.dict(m.os.environ, {'GITHUB_STEP_SUMMARY':'/summary'}), mock.patch('builtins.open', side_effect=OSError('write failed')), contextlib.redirect_stdout(io.StringIO()) as out:
            with self.assertRaises(SystemExit): m.main()
        self.assertFalse(json.loads(out.getvalue().split('FINAL_RECEIPT ')[1])['passed'])

    def test_workflow_is_exact_read_only_branch(self):
        text = (m.ROOT/'.github/workflows/guide-identity-wip.yml').read_text()
        for value in ['contents: read', 'private == false', 'persist-credentials: false', m.BRANCH, 'ubuntu-24.04']:
            self.assertIn(value, text)
        for value in ['workflow_dispatch', 'pull_request:', 'actions/cache', 'upload-artifact', 'secrets.', 'self-hosted', 'contents: write']:
            self.assertNotIn(value, text)
        runner = (HERE/'run.py').read_text()
        for command in ["['cargo'", "['sudo'", "'apt-get'", "'horizon-mixins'"]:
            self.assertNotIn(command, runner)

if __name__ == '__main__': unittest.main()
