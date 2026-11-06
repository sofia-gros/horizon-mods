"""Only source/integrity and diagnostic-parser tests; no Cargo or stage copy."""
import copy
import importlib.util
import json
from pathlib import Path
import unittest

HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location('run_game_borrow_v15', HERE / 'run_game_borrow_v15.py')
p = importlib.util.module_from_spec(spec)
spec.loader.exec_module(p)

class AppliedProbeGuards(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.seal, cls.times, cls.official = p.base_inputs()
        cls.original = (p.STAGE / p.SOURCE).read_bytes()
        cls.source = (HERE / 'run_game_borrow_v15.py').read_text()

    def test_applied_composition_is_distinct_from_pristine_evidence(self):
        self.assertEqual(len(self.seal), 5047)
        self.assertEqual(len(self.official), 19)
        self.assertEqual(self.seal[p.SOURCE], 'e02eef2d2d72b5e01b6ef9efc46de5652f33270966b2fbbb91398035d4cb3fdb')
        self.assertNotEqual(self.seal[p.SOURCE], self.official[p.SOURCE])

    def test_only_exact_added_module_diff_per_alternative(self):
        for kind in p.PROBE_SHA:
            self.assertEqual(p.proposed(self.original, kind), self.original + b'\n' + (HERE / f'game_borrow_{kind}.rs').read_bytes())
            self.assertEqual((HERE / f'UNAPPLIED-v15-game-borrow-{kind}.patch').read_text(), p.delta(self.original, kind))
            self.assertEqual(p.proposed(self.original, kind).count(b"struct Game<'a> {"), 1)

    def test_base_mtimes_and_lock_remain_exact(self):
        self.assertEqual(set(self.times), set(self.seal))
        self.assertEqual(self.seal['Cargo.lock'], '3996049ef015fefe6fb83b4d14344f2be670a2a530ed62eb5e811eaf1469765d')
        self.assertIn('files(overlay) != expected_overlay', self.source)
        self.assertIn('files(STAGE) != seal', self.source)

    def test_explicit_execution_default_has_no_copy_or_cargo(self):
        guard = self.source.index('    if not args.execute:')
        self.assertLess(guard, self.source.index('shutil.copytree('))
        self.assertLess(self.source.index('        return\n', guard), self.source.index('shutil.copytree('))
        self.assertIn("output.parent != Path('/tmp')", self.source)

    def test_alias_is_exclusive_and_finally_owner_checked(self):
        self.assertIn('if LOGICAL.exists() or LOGICAL.is_symlink():', self.source)
        self.assertIn('LOGICAL.symlink_to(overlay', self.source)
        self.assertIn('os.readlink(LOGICAL) != str(overlay)', self.source)
        self.assertNotIn('force=True', self.source)
        self.assertNotIn('rmtree', self.source)
        self.assertNotIn('os.link(', self.source)

    def test_positive_before_negative_real_lib_recheck(self):
        self.assertIn("for kind in ('positive', 'negative'):", self.source)
        self.assertIn("expected_code=0 if kind == 'positive' else 101", self.source)
        self.assertIn("('horizon_client', 'lib')", self.source)
        self.assertIn("'--lib', '--no-default-features', '--message-format=json'", self.source)
        self.assertNotIn("'--bin'", self.source)
        self.assertNotIn("'cargo', 'test'", self.source)

    def diagnostic(self):
        body = p.proposed(self.original, 'negative')
        text = body.decode()
        start = text.index('mod pluto_contact_negative_probe {')
        call = text.index('        game.add_player_effect(effect);', start)
        line = text[:call].count('\n') + 1
        byte = len(text[:call].encode()) + 8
        obj = {'reason': 'compiler-message', 'target': {'name': 'horizon_client', 'kind': ['lib']},
            'message': {'level': 'error', 'code': {'code': 'E0499'}, 'spans': [{
                'is_primary': True, 'file_name': p.SOURCE, 'line_start': line,
                'line_end': line, 'byte_start': byte, 'byte_end': byte + 4,
                'text': [{'text': '        game.add_player_effect(effect);'}]}]}}
        return obj, body

    def test_parser_accepts_only_exact_synthetic_diagnostic_shape(self):
        # Parser unit data, NOT an observed native compiler result.
        obj, body = self.diagnostic()
        self.assertEqual(p.reject_diagnostic(json.dumps(obj), body)['code'], 'E0499')

    def test_parser_rejects_unrelated_error_target_or_extra_error(self):
        obj, body = self.diagnostic()
        for mutate in [lambda o: o['message']['code'].update(code='E0308'),
                       lambda o: o['target'].update(name='horizon_client_lib'),
                       lambda o: o['target'].update(kind=['bin'])]:
            bad = copy.deepcopy(obj); mutate(bad)
            with self.assertRaises(ValueError): p.reject_diagnostic(json.dumps(bad), body)
        with self.assertRaises(ValueError): p.reject_diagnostic(json.dumps(obj)+'\n'+json.dumps(obj), body)

    def test_parser_rejects_wrong_span_even_with_expected_code(self):
        obj, body = self.diagnostic()
        for mutate in [lambda s: s.update(file_name='crates/other/src/game.rs'),
                       lambda s: s.update(line_start=1, line_end=1),
                       lambda s: s.update(byte_start=0, byte_end=4),
                       lambda s: s.update(is_primary=False),
                       lambda s: s.update(text=[{'text': 'other_call()'}])]:
            bad = copy.deepcopy(obj); mutate(bad['message']['spans'][0])
            with self.assertRaises(ValueError): p.reject_diagnostic(json.dumps(bad), body)

    def test_capacity_and_process_group_guard_preserve_cache(self):
        self.assertEqual(p.FLOOR, 10871635968)
        self.assertIn('start_new_session=True', self.source)
        self.assertIn('os.killpg(child.pid, signal.SIGINT)', self.source)
        self.assertIn('copy_function=guarded_copy', self.source)
        for forbidden in ['cargo clean', '.fingerprint/', 'touch ', "unlink(STAGE"]:
            self.assertNotIn(forbidden, self.source)

if __name__ == '__main__': unittest.main()
