"""Pure source checks only. No Rust, Cargo, official loader or CI invocation."""
import ast
import importlib.util
import json
from pathlib import Path
import re
import tempfile
import unittest

HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location('bounds_source', HERE / 'source.py')
m = importlib.util.module_from_spec(spec); spec.loader.exec_module(m)
C = json.loads((HERE / 'inputs.json').read_bytes())


class Source(unittest.TestCase):
    def test_exact_inputs_and_original_predicate(self):
        output, evidence = m.compose()
        geometry = (m.ROOT / m.GEOMETRY).read_bytes()
        providers = (m.ROOT / m.PROVIDERS).read_bytes()
        self.assertEqual(output.count(geometry), 1)
        self.assertEqual(output.count(m.extract(providers)), 1)
        self.assertEqual(evidence['prepared_tests'], 14)

    def test_only_two_safe_constructor_functions_return_batch(self):
        s = (m.ROOT / m.GEOMETRY).read_text()
        names = re.findall(r'pub fn (\w+)\([^\n]*\) -> Result<Batch, Error>', s)
        self.assertEqual(names, ['build', 'build_oriented'])
        self.assertEqual(s.count('let mut batch = Batch {'), 1)
        self.assertIn('quads: [Quad; 12],\n    len: u8,', s)
        self.assertNotRegex(s, r'\bunsafe\b|\bDerefMut\b|\bDeserialize\b|\bDefault\b')
        self.assertNotIn('pub fn push', s)
        self.assertEqual(re.findall(r'pub fn (\w+)\(&self', s), ['as_slice', 'is_empty'])
        self.assertIn('#[derive(Clone, Debug, PartialEq)]\npub struct Batch', s)

    def test_oriented_mutation_is_only_pinned_reflection(self):
        s = (m.ROOT / m.GEOMETRY).read_text()
        self.assertIn('let mut batch = build(cell)?;', s)
        self.assertIn('let plane_twice = 2.0 * f32::from(cell.origin[1]) + 1.0;', s)
        self.assertIn('vertex.position[1] = plane_twice - vertex.position[1];', s)
        self.assertEqual(s.count('for quad in &mut batch.quads'), 1)

    def test_original_bounds_have_not_been_weakened(self):
        s = m.extract((m.ROOT / m.PROVIDERS).read_bytes())
        self.assertIn(b'.chain(vertex.uv)', s)
        self.assertIn(b'value.is_finite() && (0.0..=1.0).contains(&value)', s)
        self.assertEqual(m.sha(s), C['original_predicate_sha256'])

    def test_candidate_has_complete_simple_function_body(self):
        s = (HERE / 'candidate.rs').read_text()
        self.assertTrue(s.endswith('    })\n}\n'))
        self.assertEqual(s.count('{'), s.count('}'))
        self.assertIn('vertex.position.iter().all(|&value| value <= 1.0)', s)
        self.assertNotIn('unsafe', s)

    def test_fixture_includes_nonzero_admitted_and_reflected_counterexamples(self):
        s = (HERE / 'fixture.rs').read_text()
        self.assertIn('fn nonzero_origin_can_be_local_and_top_changes_the_answer()', s)
        for token in ['c.origin = [1, 0, 0]', 'c.sides.swap(2, 3)', 'c.origin = [0, 1, 0]',
                      'f32::NAN', 'f32::NEG_INFINITY', '0x3f800001', 'f32::from_bits(1)',
                      'for mask in 0..64', 'for x in 0..16', 'let cloned = batch.clone()']:
            self.assertIn(token, s)
        self.assertIn('original::cell_local(batch)', s)
        self.assertIn('assert_eq!(candidate::cell_local(batch), expected)', s)
        self.assertNotIn('unsafe', s)

    def test_micro_measurement_is_bounded_unclaimed_and_not_asserted_fast(self):
        s = (HERE / 'fixture.rs').read_text()
        for token in ['black_box(batch)', 'for round in 0..7', 'for _ in 0..4096',
                      'ns < 1_000_000', 'clock.saturating_mul(100)',
                      'timed(original::cell_local', 'timed(candidate::cell_local']:
            self.assertIn(token, s)
        self.assertNotRegex(s, r'assert[^\n]*\b(old|new|ratio)\b')
        self.assertFalse(any(C['execution'].values()))

    def test_composition_deterministic_and_does_not_modify_modules(self):
        before = {p: (m.ROOT / p).read_bytes() for p in C['production_sources']}
        a, x = m.compose(); b, y = m.compose()
        self.assertEqual((a, x), (b, y))
        self.assertIsNone(x['performance_result'])
        self.assertEqual(before, {p: (m.ROOT / p).read_bytes() for p in before})

    def test_missing_duplicate_reversed_function_boundary_rejects(self):
        for raw in [b'', m.START + m.START + m.END, m.END + m.START,
                    m.START + b'missing terminator\n' + m.END]:
            with self.assertRaises(ValueError): m.extract(raw)

    def test_changed_hash_or_symlink_parent_rejects(self):
        with self.assertRaises(ValueError): m.read(m.ROOT, m.GEOMETRY, '0' * 64)
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp); (root / 'outside').mkdir(); (root / 'inside').mkdir()
            (root / 'outside/x').write_bytes(b'x')
            (root / 'inside/link').symlink_to(root / 'outside', target_is_directory=True)
            with self.assertRaisesRegex(ValueError, 'symlink'):
                m.read(root / 'inside', 'link/x', m.sha(b'x'))

    def test_source_composer_never_invokes_tools_or_writes(self):
        tree = ast.parse((HERE / 'source.py').read_text())
        imports = {n.module for n in ast.walk(tree) if isinstance(n, ast.ImportFrom)}
        imports |= {a.name for n in ast.walk(tree) if isinstance(n, ast.Import) for a in n.names}
        self.assertFalse(imports & {'subprocess', 'shutil', 'os'})
        calls = {n.func.attr for n in ast.walk(tree) if isinstance(n, ast.Call) and isinstance(n.func, ast.Attribute)}
        self.assertFalse(calls & {'write_bytes', 'write_text', 'mkdir', 'unlink', 'chmod', 'rename'})


if __name__ == '__main__':
    unittest.main()
