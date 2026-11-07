"""Pure/mock coverage of a single explicit Cargo concurrency setting."""
import ast
import hashlib
import json
from pathlib import Path
import subprocess
import tempfile
import unittest

from test_windows_link_only import FakePreparedWindows, ROOT, m
from windows_source_layers import before_jobs_one, undo


class CapturedWindows(FakePreparedWindows):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)
        self.environments = []

    def run(self, argv, **kwargs):
        self.environments.append((list(map(str, argv)), dict(kwargs['env'])))
        return super().run(argv, **kwargs)


class Tests(unittest.TestCase):
    def test_inherited_jobs_and_other_build_overrides_are_scrubbed_before_one(self):
        original = {'PATH': 'existing-path', 'CARGO_BUILD_JOBS': '999', 'cargo_build_jobs': '7',
                    'CARGO_BUILD_RUSTC_WRAPPER': 'untrusted', 'Keep': 'value'}
        saved = dict(original)
        env = m.build_environment(original, Path('/tools'), Path('/home'))
        self.assertEqual(original, saved)
        self.assertEqual(env['CARGO_BUILD_JOBS'], '1')
        self.assertNotIn('cargo_build_jobs', env)
        self.assertNotIn('CARGO_BUILD_RUSTC_WRAPPER', env)
        self.assertEqual(env['Keep'], 'value')
        self.assertEqual([k for k in env if k.upper().startswith('CARGO_BUILD_')], ['CARGO_BUILD_JOBS'])

    def test_old_environment_diff_is_only_explicit_jobs_for_all_prior_inputs(self):
        raw = before_jobs_one('tools/play_windows_latest.py', (ROOT/'tools/play_windows_latest.py').read_bytes())
        tree = ast.parse(raw)
        function = next(n for n in tree.body if isinstance(n, ast.FunctionDef) and n.name == 'build_environment')
        namespace = dict(m.__dict__)
        exec(compile(ast.Module(body=[function], type_ignores=[]), '<exact-prior-function>', 'exec'), namespace)
        for original in ({}, {'PATH': 'path'}, {'CARGO_BUILD_JOBS': '0', 'cargo_build_jobs': 'bad', 'RUSTFLAGS': '-bad'}):
            with self.subTest(original=original):
                prior = namespace['build_environment'](original, Path('/tools'), Path('/home'))
                current = m.build_environment(original, Path('/tools'), Path('/home'))
                self.assertNotIn('CARGO_BUILD_JOBS', prior)
                self.assertEqual(current.pop('CARGO_BUILD_JOBS'), '1')
                self.assertEqual(current, prior)

    def test_same_jobs_environment_reaches_guarded_stage_and_locked_default_link(self):
        with tempfile.TemporaryDirectory() as temp:
            fake = CapturedWindows(Path(temp)); fake.execute()
            stage = [(a,e) for a,e in fake.environments if len(a)>1 and a[1].endswith('/tools/stage.py')]
            link = [(a,e) for a,e in fake.environments if len(a)>1 and a[1]=='build']
            self.assertEqual(len(stage), 1); self.assertEqual(len(link), 1)
            self.assertEqual(stage[0][1], link[0][1])
            self.assertEqual(stage[0][1]['CARGO_BUILD_JOBS'], '1')
            self.assertIn('--locked', link[0][0]); self.assertIn('--offline', link[0][0])
            self.assertNotIn('--no-default-features', link[0][0])
            self.assertFalse(any('--tconstruct-prepare' in a for a,e in fake.environments))

    def test_stage_failure_does_not_reach_client_compile(self):
        with tempfile.TemporaryDirectory() as temp:
            fake = CapturedWindows(Path(temp), fail='guard')
            with self.assertRaises(subprocess.CalledProcessError): fake.execute()
            self.assertTrue(any(len(a)>1 and a[1].endswith('/tools/stage.py') for a,e in fake.environments))
            self.assertFalse(any(len(a)>1 and a[1]=='build' for a,e in fake.environments))
            self.assertTrue(all(e['CARGO_BUILD_JOBS']=='1' for a,e in fake.environments))

    def test_stage_children_inherit_environment_without_override(self):
        tree = ast.parse((ROOT/'tools/stage.py').read_bytes())
        calls = [n for n in ast.walk(tree) if isinstance(n, ast.Call) and isinstance(n.func, ast.Attribute)
                 and isinstance(n.func.value, ast.Name) and n.func.value.id=='subprocess']
        self.assertEqual(len(calls), 8)
        self.assertTrue(all(not any(k.arg in ('env', None) for k in n.keywords) for n in calls))
        runs = [n for n in calls if n.func.attr=='run']
        self.assertEqual(len(runs), 4)
        self.assertTrue(all(any(k.arg=='check' and isinstance(k.value, ast.Constant) and k.value.value is True
                                for k in n.keywords) for n in runs))

    def test_whole_source_and_historical_adapters_reverse_exactly(self):
        spec = json.loads((ROOT/'experiments/windows-jobs-one/inverse.json').read_bytes())
        self.assertEqual(set(spec['files']), {'tools/play_windows_latest.py'})
        for rel, row in spec['files'].items():
            raw = (ROOT/rel).read_bytes()
            self.assertEqual(hashlib.sha256(undo(raw,row)).hexdigest(), row['before_sha256'])
            with self.assertRaises(AssertionError): undo(raw+b'\n', row)
        for rel, row in spec['historical_test_adapters'].items():
            self.assertEqual(hashlib.sha256(undo((ROOT/'tests'/rel).read_bytes(),row)).hexdigest(), row['before_sha256'])


if __name__ == '__main__': unittest.main()
