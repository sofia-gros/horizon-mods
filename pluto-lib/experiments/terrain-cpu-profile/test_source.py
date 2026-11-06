"""Read-only source checks; no native execution, output tree or compiler."""
import ast
import importlib.util
import json
import os
from pathlib import Path
import re
import unittest

HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location('profile_source', HERE/'source.py')
m = importlib.util.module_from_spec(spec);spec.loader.exec_module(m)
OFFICIAL = Path(os.environ.get('PLUTO_OFFICIAL_SOURCE', m.ROOT.parents[1]/'.devtools/latest-official-export-20261105/horizon'))

class Source(unittest.TestCase):
    def test_exact_pins_and_one_extra_test(self):
        c = m.validate()
        self.assertEqual(len(c['source_sha256']),12)
        s = (HERE/'profile.rs').read_text()
        self.assertEqual(re.findall(r'#\[test\]\s*fn (\w+)\(',s), ['same_output_callbacks_and_microkernels'])
        self.assertNotIn('#[ignore]',s);self.assertNotIn('#![',s)

    def test_exact_insertion_inverse(self):
        t = (m.BASE/'fixture.rs').read_bytes();extra = (HERE/'profile.rs').read_bytes()
        result = m.extend(t,extra)
        self.assertEqual(m.remove(result),t)
        self.assertEqual(result.count(extra),1)
        self.assertEqual(result.count(b'fn real_native_mesh_differential_and_timing()'),1)

    def test_lean_removes_only_two_original_assertions(self):
        old = (m.BASE/'fixture.rs').read_text()
        a = old.index('    fn provider(');b = old.index('    struct Fixture {',a)
        expected = old[a:b]
        for line in [
            '        assert_eq!(context.neighborhood.binding([0; 3]), Some(context.binding));\n',
            '        assert!(context.neighborhood.binding([2, 0, 0]).is_none());\n',
        ]:
            self.assertEqual(expected.count(line),1);expected = expected.replace(line,'')
        expected = expected.replace('fn provider(','fn lean(',1)
        expected = ''.join('    '+line if line.strip() else line for line in expected.splitlines(keepends=True))
        new = (HERE/'profile.rs').read_text();a = new.index('        fn lean(');b = new.index('        fn cloned(',a)
        self.assertEqual(new[a:b],expected)

    def test_ambiguous_empty_or_unbounded_insertions_reject(self):
        t = (m.BASE/'fixture.rs').read_bytes()
        for template in [b'', t+t, t+m.START, t+m.END]:
            with self.assertRaises(ValueError):m.extend(template,b'extra')
        for extra in [b'',b'x'*32769,m.START,m.END,m.MARKER]:
            with self.assertRaises(ValueError):m.extend(t,extra)
        for source in [b'',m.START+m.START+m.END,m.END+m.START]:
            with self.assertRaises(ValueError):m.remove(source)

    def test_real_native_full_body_inverse_in_memory(self):
        extended,evidence = m.render(OFFICIAL)
        spec = importlib.util.spec_from_file_location('original_prepare', m.BASE/'prepare.py')
        base = importlib.util.module_from_spec(spec);spec.loader.exec_module(base)
        original,meta = base.baseline(OFFICIAL)
        self.assertEqual(m.remove(extended),original)
        self.assertEqual(evidence['baseline_generated_sha256'],m.sha(original))
        self.assertEqual(evidence['native_function_sha256'],meta['function_sha256'])
        self.assertEqual(extended.count(b'fn native_baseline('),1)

    def test_old_process_modes_and_new_process_are_separate(self):
        old = (m.BASE/'fixture.rs').read_text();new = (HERE/'profile.rs').read_text()
        self.assertIn('mode == "no-provider" || mode == "registered"',old)
        self.assertIn('"PLUTO_TERRAIN_FIXTURE_MODE").unwrap(), "profile"',new)
        self.assertNotIn('real_native_mesh_differential_and_timing()',new)
        self.assertIn('"opt0-assertions" | "opt3-assertions"',new)

    def test_all_callbacks_freeze_before_native_mesh(self):
        s = (HERE/'profile.rs').read_text().split('fn same_output_callbacks_and_microkernels()',1)[1]
        self.assertLess(s.index('FIXED.set('),s.index('cells::register_all('))
        self.assertLess(s.index('cells::register_materials('),s.index('let ordinary ='))
        self.assertEqual(s.count('cells::register_'),2)
        self.assertIn('context.binding.variant != 1',(HERE/'profile.rs').read_text())
        self.assertIn('context.appearance.anchor != geometry::VerticalAnchor::Bottom',(HERE/'profile.rs').read_text())

    def test_direct_and_full_output_checks_are_before_timing(self):
        s = (HERE/'profile.rs').read_text()
        for token in ['assert_eq!(original, lean(context))','assert_eq!(original, cloned(context))',
            'assert_eq!(original.geometry(), other.geometry())','assert_eq!(original.slot(i), other.slot(i))',
            'scratch_equal(&sa, &sb','oa.sort_solid(); ob.sort_solid(); equal(&oa, &ob)',
            '256 * 6 * 4','(8., 8.5)']:
            self.assertIn(token,s)
        pair = s.split('fn pair(',1)[1].split('fn micro<',1)[0]
        self.assertLess(pair.index('equivalent(a, b'),pair.index('duration(a,'))
        self.assertNotIn('checked_geometry',pair);self.assertNotIn('checked_material',pair)

    def test_microkernels_use_actual_helpers_and_fixed_schedule(self):
        s = (HERE/'profile.rs').read_text()
        for token in ['geometry::build(black_box(&shape))','geometry::Batch::clone(black_box(batch))',
            'cells::cell_local(black_box(batch))','cells::uv_subset(black_box(&source), black_box(&inside))',
            'cells::uv_subset(black_box(&source), black_box(&outside))','for _ in 0..4096',
            'for round in 0..7','let index = (round + offset) % 5']:
            self.assertIn(token,s)

    def test_inconclusive_timing_is_explicit(self):
        s = (HERE/'profile.rs').read_text()
        self.assertEqual(s.count('timing_inconclusive_short_samples'),2)
        self.assertIn('"guaranteed_clock_resolution":false',s)
        self.assertIn('(x[3] != 0).then(',s)
        self.assertIn('left_total_ns',s);self.assertIn('right_total_ns',s)
        self.assertNotIn('assert!(ratio',s)

    def test_source_composer_cannot_write_or_launch_tools(self):
        tree = ast.parse((HERE/'source.py').read_text())
        imports = {n.module for n in ast.walk(tree) if isinstance(n,ast.ImportFrom)}
        imports |= {a.name for n in ast.walk(tree) if isinstance(n,ast.Import) for a in n.names}
        self.assertFalse(imports & {'subprocess','os','shutil'})
        calls = {n.func.attr for n in ast.walk(tree) if isinstance(n,ast.Call) and isinstance(n.func,ast.Attribute)}
        self.assertFalse(calls & {'write_bytes','write_text','mkdir','unlink','chmod','open','rename'})
        self.assertIn('Unexecuted Rust source',(HERE/'README.md').read_text())

if __name__ == '__main__':unittest.main()
