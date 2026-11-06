"""Pure in-memory inverse checks; no compiler, planner or installed changes."""
import ast
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest

HERE=Path(__file__).resolve().parent
spec=importlib.util.spec_from_file_location('host_bounds',HERE/'compose.py')
m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m)
C=json.loads((HERE/'inputs.json').read_text())


class Source(unittest.TestCase):
    def inputs(self):
        return tuple(m.read(m.ROOT,p,C['source_sha256'][p]) for p in (m.HOST,m.CANDIDATE))

    def test_exact_host_and_candidate_compose_and_invert(self):
        host,candidate=self.inputs();generated,r=m.compose()
        self.assertEqual(m.inverse(generated,candidate,m.sha(host)),host)
        self.assertEqual(r['changed_call_sites'],1)
        self.assertFalse(any(r['execution'].values()))
        self.assertFalse(r['manifest_created']);self.assertIsNone(r['performance_result'])

    def test_only_one_validation_call_and_candidate_append_change(self):
        host,candidate=self.inputs();generated,_=m.compose()
        self.assertEqual(generated,host.replace(m.BEFORE,m.AFTER,1)+m.MODULE+candidate+b'}\n')
        self.assertNotIn(m.BEFORE,generated)
        self.assertEqual(generated.count(m.AFTER),1)
        self.assertIn(b'pub(super) fn mesh(',host)
        self.assertIn(b'!api::cell_local(batch) || !budget.quads(batch.as_slice().len())',host)
        self.assertIn(b'!local_bounds_candidate::cell_local(batch) || !budget.quads(batch.as_slice().len())',generated)

    def test_deterministic_without_writing_original_sources(self):
        before={p:(m.ROOT/p).read_bytes() for p in C['source_sha256']}
        self.assertEqual(m.compose(),m.compose())
        self.assertEqual(before,{p:(m.ROOT/p).read_bytes() for p in before})

    def test_missing_duplicate_or_preexisting_candidate_calls_reject(self):
        host,candidate=self.inputs()
        for bad in [host.replace(m.BEFORE,b'other(batch)'),host+b'\n'+m.BEFORE,
                    host+b'\nlocal_bounds_candidate',host.replace(m.BEFORE,m.AFTER)]:
            with self.assertRaises(ValueError):m.transform(bad,candidate)

    def test_inverse_rejects_change_outside_validation_predicate(self):
        host,candidate=self.inputs();generated=m.transform(host,candidate)
        for bad in [generated.replace(b'budget.quads',b'budget.other',1),
                    generated.replace(m.AFTER,m.BEFORE,1),generated+b'\n',generated[:-1]]:
            with self.assertRaises(ValueError):m.inverse(bad,candidate,m.sha(host))

    def test_candidate_signature_body_and_budgets_reject(self):
        host,candidate=self.inputs()
        for bad in [candidate[:-2],candidate.replace(b'&super::geometry::Batch',b'&Foreign'),
                    candidate+b'\nmod extra {}\n',b'#![allow(unused)]\n'+candidate]:
            with self.assertRaises(ValueError):m.transform(host,bad)
        with self.assertRaises(ValueError):m.transform(b'x'*(m.LIMIT+1),candidate)

    def test_changed_sources_and_parent_symlinks_reject(self):
        with self.assertRaises(ValueError):m.read(m.ROOT,m.HOST,'0'*64)
        with tempfile.TemporaryDirectory() as tmp:
            root=Path(tmp);(root/'outside').mkdir();(root/'inside').mkdir()
            (root/'outside/x').write_bytes(b'x');(root/'inside/link').symlink_to(root/'outside',target_is_directory=True)
            with self.assertRaises(ValueError):m.read(root/'inside','link/x',m.sha(b'x'))
            for p in ['/x','../x','a/../x','a\\x','a//x']:
                with self.assertRaises(ValueError):m.read(root,p,m.sha(b'x'))

    def test_no_launch_write_or_guard_update_capability(self):
        tree=ast.parse((HERE/'compose.py').read_text())
        imports={n.module for n in ast.walk(tree) if isinstance(n,ast.ImportFrom)}
        imports|={a.name for n in ast.walk(tree) if isinstance(n,ast.Import) for a in n.names}
        self.assertFalse(imports & {'os','subprocess','shutil','requests'})
        calls={n.func.attr for n in ast.walk(tree) if isinstance(n,ast.Call) and isinstance(n.func,ast.Attribute)}
        self.assertFalse(calls & {'write_text','write_bytes','mkdir','unlink','chmod','rename'})


if __name__ == '__main__':unittest.main()
