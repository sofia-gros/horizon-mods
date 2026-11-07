"""Pure child-encoding contract tests; no child process or formatter is run."""
import ast
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import sys
import tempfile
import unittest
from windows_source_layers import before_package_contract

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / 'tools'))
import play_windows_latest as launch

EVIDENCE = json.loads((ROOT / 'tests/windows-python-utf8-evidence.json').read_text(encoding='utf-8'))
def sha(raw): return hashlib.sha256(raw).hexdigest()


class Tests(unittest.TestCase):
    def test_child_environment_overrides_both_encodings_case_insensitively(self):
        original = {'PATH': 'original', 'PYTHONUTF8': '0', 'pythonutf8': '0',
                    'PYTHONIOENCODING': 'cp1252:replace', 'PythonIoEncoding': 'ascii'}
        before = original.copy()
        actual = launch.build_environment(original, Path('tool'), Path('home'))
        self.assertEqual(original, before)
        self.assertEqual(actual['PYTHONUTF8'], '1')
        self.assertEqual(actual['PYTHONIOENCODING'], 'utf-8')
        self.assertEqual([k for k in actual if k.upper() == 'PYTHONUTF8'], ['PYTHONUTF8'])
        self.assertEqual([k for k in actual if k.upper() == 'PYTHONIOENCODING'], ['PYTHONIOENCODING'])

    def test_only_two_process_encoding_assignments_are_added(self):
        raw = before_package_contract('tools/play_windows_latest.py', (ROOT / 'tools/play_windows_latest.py').read_bytes(), before_stat=True)
        text = raw.decode('utf-8')
        inverse = json.loads((ROOT / 'tests/windows-link-only-inverse.json').read_text(encoding='utf-8'))['files']['tools/play_windows_latest.py']
        self.assertEqual(sha(raw), inverse['after_sha256'])
        for old, new in reversed(inverse['edits']):
            self.assertEqual(text.count(new), 1); text = text.replace(new, old, 1)
        self.assertEqual(sha(text.encode('utf-8')), EVIDENCE['launcher_sha256'])
        after = "'PYTHONUTF8', 'PYTHONIOENCODING', 'PLUTO_GUARD_RUSTFMT', 'CARGO'"
        self.assertEqual(text.count(after), 1)
        text = text.replace(after, "'PLUTO_GUARD_RUSTFMT', 'CARGO'", 1)
        addition = "\n               PYTHONUTF8='1', PYTHONIOENCODING='utf-8',"
        self.assertEqual(text.count(addition), 1)
        text = text.replace(addition, '', 1)
        self.assertEqual(sha(text.encode('utf-8')), EVIDENCE['previous_launcher_sha256'])

    def test_actual_official_source_observations_are_fixed(self):
        self.assertEqual(EVIDENCE['official'], '41f822b659789780f58d32c63ddfb6887ea1b8bf')
        expected = [
            (1032243, 13, '2488718332e589c8cbc15766c93857e83945b5f90b74f393ceaa1cd75c23891e',
             '1bc4a9c8ce25aead058e60f8a020f94c1ade9e441fd280b917a509ccb851363a'),
            (383916, 5, '3b4e83cb752575fb47821a37575dc9dd277ceb3630b0fc6a00c2e621c9e54cef',
             '5ec9b7a848c6cbcead593a2c8642c599c75db1cefcb9064bbceb9e9966517224'),
        ]
        self.assertEqual(len(EVIDENCE['official_sources']), len(expected))
        for row, want in zip(EVIDENCE['official_sources'], expected):
            self.assertEqual((row['bytes'], row['non_ascii_lines'], row['sha256'],
                              row['cp1252_reencoded_sha256']), want)
            self.assertEqual(len(row['samples']), want[1])
            self.assertEqual(len({s['line'] for s in row['samples']}), want[1])

    def test_actual_unicode_lines_reproduce_cp1252_hash_drift(self):
        # These are exact lines from the pinned official files, not generated
        # Unicode examples. Whole-file observations are retained separately.
        for row in EVIDENCE['official_sources']:
            for sample in row['samples']:
                raw = bytes.fromhex(sample['utf8_hex'])
                self.assertTrue(any(b > 127 for b in raw))
                self.assertTrue(raw.endswith(b'\n'))
                self.assertEqual(raw.decode('utf-8').encode('utf-8'), raw)
                self.assertNotEqual(raw.decode('cp1252').encode('utf-8'), raw)

    def test_unchanged_damage_digest_requires_correct_decoding(self):
        path = ROOT.parent / 'pluto-lib/tools/check_damage_equipment_plan.py'
        self.assertEqual(sha(path.read_bytes()), EVIDENCE['unmodified_damage_checker_sha256'])
        spec = importlib.util.spec_from_file_location('damage_encoding_test', path)
        checker = importlib.util.module_from_spec(spec); spec.loader.exec_module(checker)
        with tempfile.TemporaryDirectory() as tmp:
            p = Path(tmp) / 'official-lines.rs'
            for row in EVIDENCE['official_sources']:
                raw = b''.join(bytes.fromhex(s['utf8_hex']) for s in row['samples'])
                p.write_bytes(raw)
                self.assertEqual(checker.digest(p.read_text(encoding='utf-8')), sha(raw))
                self.assertNotEqual(checker.digest(p.read_text(encoding='cp1252')), sha(raw))

    def test_stage_and_guard_inherit_the_selected_child_environment(self):
        tree = ast.parse((ROOT / 'tools/play_windows_latest.py').read_text(encoding='utf-8'))
        stage_calls = [n for n in ast.walk(tree) if isinstance(n, ast.Call)
                       and isinstance(n.func, ast.Name) and n.func.id == 'run'
                       and n.args and 'tools/stage.py' in ast.unparse(n.args[0])]
        self.assertEqual(len(stage_calls), 1)
        self.assertEqual(ast.unparse(next(k.value for k in stage_calls[0].keywords if k.arg == 'env')), 'env')
        args = ast.unparse(stage_calls[0].args[0])
        self.assertIn('sys.executable', args)
        for bypass in ["'-E'", "'-I'", "'utf8=0'"]: self.assertNotIn(bypass, args)
        raw = (ROOT / 'tools/stage.py').read_bytes()
        self.assertEqual(sha(before_package_contract('tools/stage.py', raw)), EVIDENCE['unmodified_stage_sha256'])
        tree = ast.parse(raw)
        guard_calls = [n for n in ast.walk(tree) if isinstance(n, ast.Call)
                       and isinstance(n.func, ast.Attribute) and n.func.attr == 'run'
                       and n.args and 'pluto-lib/tools/check_native_plan.py' in ast.unparse(n.args[0])]
        self.assertEqual(len(guard_calls), 1)
        self.assertFalse(any(k.arg == 'env' for k in guard_calls[0].keywords))
        self.assertIn('sys.executable', ast.unparse(guard_calls[0].args[0]))

    def test_parent_process_environment_and_system_settings_are_not_changed(self):
        original = dict(os.environ)
        launch.build_environment(os.environ, Path('tool'), Path('home'))
        self.assertEqual(dict(os.environ), original)
        tree = ast.parse((ROOT / 'tools/play_windows_latest.py').read_text(encoding='utf-8'))
        function = next(n for n in tree.body if isinstance(n, ast.FunctionDef) and n.name == 'build_environment')
        self.assertFalse(any(isinstance(n, ast.Subscript) and isinstance(n.ctx, ast.Store)
                             and ast.unparse(n.value) == 'os.environ' for n in ast.walk(function)))


if __name__ == '__main__': unittest.main()
