"""Probe reuse must save work without caching guard decisions."""
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'tools/smoke-local'))
from probe_cache import guarded_plan, record_build, reusable_build, sync_tree, tree_digest, fresh_output


class ProbeCache(unittest.TestCase):
    def test_plan_reuses_only_matching_bytes_and_always_runs_guard(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            plan, receipt = root / 'plan.json', root / 'receipt.json'
            calls = []
            def produce():
                calls.append('plan')
                plan.write_text('{}\n', encoding='utf-8', newline='\n')
            def guard():
                calls.append('guard')
            key = {'input': 'one'}
            self.assertFalse(guarded_plan(plan, receipt, key, produce, guard))
            self.assertTrue(guarded_plan(plan, receipt, key, produce, guard))
            self.assertEqual(calls, ['plan', 'guard', 'guard'])
            plan.write_bytes(b'tampered')
            self.assertFalse(guarded_plan(plan, receipt, key, produce, guard))
            def reject():
                raise ValueError('changed guard rejected composition')
            with self.assertRaises(ValueError):
                guarded_plan(plan, receipt, key, produce, reject)
            self.assertFalse(receipt.exists())
            self.assertFalse(guarded_plan(plan, receipt, key, produce, guard))
            self.assertFalse(guarded_plan(plan, receipt, {'input': 'two'}, produce, guard))

    def test_build_rejects_changed_inputs_and_artifacts(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            receipt = root / 'receipt.json'
            for name in ('horizon-engine', 'horizon-ctl', 'native-mods.json', 'runtime.dll'):
                (root / name).write_bytes(b'original')
            key = {'bundles': 'one'}
            record_build(receipt, key, root)
            self.assertTrue(reusable_build(receipt, key, root))
            self.assertFalse(reusable_build(receipt, {'bundles': 'two'}, root))
            self.assertFalse(reusable_build(receipt, None, root))
            for name in ('horizon-engine', 'horizon-ctl', 'native-mods.json', 'runtime.dll'):
                path = root / name
                path.write_bytes(b'changed')
                self.assertFalse(reusable_build(receipt, key, root), name)
                path.write_bytes(b'original')
            (root / 'horizon-ctl').unlink()
            self.assertFalse(reusable_build(receipt, key, root))

    def test_rebuild_uses_fresh_path_and_restores_previous_output_on_failure(self):
        with tempfile.TemporaryDirectory() as temp:
            output = Path(temp) / 'out'
            output.mkdir()
            (output / 'engine').write_bytes(b'old')
            with self.assertRaises(SystemExit):
                with fresh_output(output):
                    self.assertFalse(output.exists())
                    output.mkdir()
                    (output / 'engine').write_bytes(b'partial')
                    raise SystemExit('failed build')
            self.assertEqual((output / 'engine').read_bytes(), b'old')
            with fresh_output(output):
                self.assertFalse(output.exists())
                output.mkdir()
                (output / 'engine').write_bytes(b'new')
            self.assertEqual((output / 'engine').read_bytes(), b'new')
            backup = Path(temp) / 'out.previous'
            self.assertFalse(backup.exists())
            backup.mkdir()
            with self.assertRaises(ValueError):
                with fresh_output(output):
                    self.fail('must refuse a pre-existing backup')
            self.assertEqual((output / 'engine').read_bytes(), b'new')

    def test_stable_sync_removes_stale_files_without_touching_equal_files(self):
        with tempfile.TemporaryDirectory() as temp:
            source, target = Path(temp) / 'source', Path(temp) / 'target'
            source.mkdir()
            (source / 'input.rs').write_bytes(b'first')
            (source / 'target').mkdir()
            (source / 'target/cache').write_bytes(b'ignored')
            sync_tree(source, target)
            stamp = (target / 'input.rs').stat().st_mtime_ns
            digest = tree_digest(source)
            (target / 'stale').write_bytes(b'remove')
            sync_tree(source, target)
            self.assertEqual((target / 'input.rs').stat().st_mtime_ns, stamp)
            self.assertFalse((target / 'stale').exists())
            self.assertFalse((target / 'target').exists())
            (source / 'target/cache').write_bytes(b'new ignored bytes')
            self.assertEqual(tree_digest(source), digest)
            (source / 'input.rs').write_bytes(b'second')
            self.assertNotEqual(tree_digest(source), digest)
            sync_tree(source, target)
            self.assertEqual((target / 'input.rs').read_bytes(), b'second')
