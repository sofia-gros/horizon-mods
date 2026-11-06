"""Pure source/inverse tests. No compiler, Mixins or generated output directory."""
import ast
import importlib.util
import json
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location('latest_compose', HERE/'compose.py')
m = importlib.util.module_from_spec(spec); spec.loader.exec_module(m)
ROOT = Path(os.environ['PLUTO_MOD_SOURCE']) if 'PLUTO_MOD_SOURCE' in os.environ else HERE.parents[2]
LATEST = Path(os.environ['PLUTO_LATEST_SELECTED_SOURCE'])
C = json.loads((HERE/'inputs.json').read_bytes())


class SourceTests(unittest.TestCase):
    def test_exact_latest_and_frozen_source_pins(self):
        self.assertEqual(len(C['selected_latest_sources']), 23)
        self.assertEqual(len(C['frozen_mod_sources']), 18)
        for path, sha in C['selected_latest_sources'].items(): m.read(LATEST, path, sha)
        for path, sha in C['frozen_mod_sources'].items(): m.read(ROOT/'pluto-lib', path, sha)

    def test_original_full_body_oracle_and_inverse(self):
        result, evidence = m.compose(ROOT, LATEST)
        path = ROOT/'pluto-lib/experiments/terrain-material-cpu/prepare.py'
        spec = importlib.util.spec_from_file_location('old_baseline', path)
        old = importlib.util.module_from_spec(spec); spec.loader.exec_module(old)
        generated, meta = old.baseline(LATEST)
        self.assertEqual(m.remove(m.remove(result, 'reload'), 'profile'), generated)
        self.assertEqual(evidence['baseline_generated_sha256'], m.sha(generated))
        self.assertEqual(evidence['native_function_sha256'], meta['function_sha256'])
        self.assertEqual(result.count(b'fn native_baseline('), 1)

    def test_composition_is_deterministic_and_is_not_execution(self):
        first, a = m.compose(ROOT, LATEST); second, b = m.compose(ROOT, LATEST)
        self.assertEqual((first, a), (second, b))
        self.assertEqual(a['status'], 'source_composed_only')
        self.assertFalse(any(a['execution'].values()))
        self.assertFalse(a['compiler_profiles_observed'])
        self.assertIsNone(a['compatibility'])

    def test_three_source_tests_and_six_reload_cases(self):
        _, result = m.compose(ROOT, LATEST)
        self.assertEqual(len(result['prepared_rust_tests']), 3)
        self.assertEqual(result['prepared_reload_cases'], 6)
        self.assertEqual(set(result['prepared_rust_tests']), set(C['expected_tests']))

    def test_independent_add_remove_orders_preserve_template(self):
        template = (ROOT/'pluto-lib'/m.TEMPLATE).read_bytes()
        for order in [('profile', 'reload'), ('reload', 'profile')]:
            value = template
            for kind in order: value = m.add(value, ('mod '+kind+' {}').encode(), kind)
            for kind in reversed(order): value = m.remove(value, kind)
            self.assertEqual(value, template)

    def test_ambiguous_or_oversized_extensions_fail_closed(self):
        template = b'    '+m.MARKER+b'\n'
        for value in [b'', template+template, b' '+m.MARKER+b'\n']:
            with self.assertRaises(ValueError): m.add(value, b'mod x {}', 'profile')
        for fragment in [b'', b'x'*(m.MAX_FRAGMENT+1), m.MARKER,
                         m.boundary('reload'), m.boundary('profile', True)]:
            with self.assertRaises(ValueError): m.add(template, fragment, 'profile')
        with self.assertRaises(ValueError): m.add(m.add(template,b'x','profile'),b'x','profile')
        with patch.object(m, 'MAX_OUTPUT', 1):
            with self.assertRaises(ValueError): m.add(template,b'x','profile')

    def test_missing_duplicate_or_reversed_boundaries_fail(self):
        for value in [b'', m.boundary('profile')*2+m.boundary('profile',True),
                      m.boundary('profile',True)+m.boundary('profile')]:
            with self.assertRaises(ValueError): m.remove(value,'profile')
        with self.assertRaises(ValueError): m.boundary('unexpected')

    def test_real_native_source_drift_rejects(self):
        meta = json.loads((ROOT/'pluto-lib'/m.META).read_bytes())
        template = (ROOT/'pluto-lib'/m.TEMPLATE).read_bytes()
        native = (LATEST/meta['file']).read_bytes()
        with self.assertRaises(ValueError): m.baseline(template,native+b'\n',meta)
        changed = dict(meta, function_sha256='0'*64)
        with self.assertRaises(ValueError): m.baseline(template,native,changed)
        with self.assertRaises(ValueError): m.baseline(template+template,native,meta)

    def test_pinned_mod_or_selected_source_mismatch_rejects(self):
        path = next(iter(C['selected_latest_sources']))
        with self.assertRaises(ValueError): m.read(LATEST,path,'0'*64)
        path = next(iter(C['frozen_mod_sources']))
        with self.assertRaises(ValueError): m.read(ROOT/'pluto-lib',path,'0'*64)
        with self.assertRaises(ValueError): m.read(ROOT,'../anything','0'*64)

    def test_parent_or_root_symlink_cannot_escape_selected_root(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory); (root/'outside').mkdir(); (root/'source').mkdir()
            (root/'outside/item').write_bytes(b'correct pinned bytes')
            digest = m.sha(b'correct pinned bytes')
            (root/'source/alias').symlink_to(root/'outside', target_is_directory=True)
            with self.assertRaisesRegex(ValueError, 'symlink source component'):
                m.read(root/'source','alias/item',digest)
            (root/'root-alias').symlink_to(root/'outside', target_is_directory=True)
            with self.assertRaisesRegex(ValueError, 'symlink source component'):
                m.read(root/'root-alias','item',digest)
            (root/'source/direct').symlink_to(root/'outside/item')
            with self.assertRaisesRegex(ValueError, 'symlink source component'):
                m.read(root/'source','direct',digest)

    def test_flags_are_only_plans_not_claimed_compiler_evidence(self):
        self.assertEqual(C['proposed_profiles']['opt0-assertions']['opt_level'],0)
        self.assertEqual(C['proposed_profiles']['opt3-assertions']['opt_level'],3)
        for row in C['proposed_profiles'].values():
            self.assertIs(row['debug_assertions'], True)
            self.assertIs(row['overflow_checks'], True)
        self.assertFalse(any(C['execution'].values()))

    def test_composer_cannot_launch_tools_or_write_output(self):
        tree = ast.parse((HERE/'compose.py').read_text())
        imports = {n.module for n in ast.walk(tree) if isinstance(n,ast.ImportFrom)}
        imports |= {a.name for n in ast.walk(tree) if isinstance(n,ast.Import) for a in n.names}
        self.assertFalse(imports & {'subprocess','os','shutil'})
        calls = {n.func.attr for n in ast.walk(tree) if isinstance(n,ast.Call) and isinstance(n.func,ast.Attribute)}
        self.assertFalse(calls & {'write_bytes','write_text','mkdir','unlink','chmod','rename','run','Popen'})
        for call in ast.walk(tree):
            if isinstance(call,ast.Call) and isinstance(call.func,ast.Attribute) and call.func.attr=='open':
                self.assertEqual(ast.literal_eval(call.args[0]),'rb')


if __name__ == '__main__': unittest.main()
