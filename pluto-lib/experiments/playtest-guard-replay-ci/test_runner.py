"""Pure replay contract checks; no packaging, Git, native or external process."""
import importlib.util,json,tempfile,unittest,ast,copy
from pathlib import Path
from unittest.mock import patch
HERE=Path(__file__).resolve().parent
s=importlib.util.spec_from_file_location('replay',HERE/'run.py');r=importlib.util.module_from_spec(s);s.loader.exec_module(r)
class Tests(unittest.TestCase):
 def test_public_exact_event(self):
  e={'GITHUB_ACTIONS':'true','GITHUB_REPOSITORY':r.prior.REPOSITORY,'GITHUB_EVENT_NAME':'push','GITHUB_REF':'refs/heads/'+r.BRANCH};self.assertTrue(r.event_allowed(e,{'repository':{'private':False}}));e['GITHUB_EVENT_NAME']='pull_request';self.assertFalse(r.event_allowed(e,{'repository':{'private':False}}))
 def test_private_event_denied(self):self.assertFalse(r.event_allowed({'GITHUB_ACTIONS':'true'},{'repository':{'private':True}}))
 def test_head_fixed(self):
  c=r.inputs();c['integration']['head']='a'*40
  with self.assertRaises(ValueError):r.validate(c)
 def test_tree_fixed(self):
  c=r.inputs();c['integration']['tree']='b'*40
  with self.assertRaises(ValueError):r.validate(c)
 def test_source_set_fixed(self):
  c=r.inputs();c['source_sha256']['extra']='c'*64
  with self.assertRaises(ValueError):r.validate(c)
 def test_inputs_exact(self):
  m={'Cargo.lock':'a'*64};self.assertEqual(r.verify_inputs({'inputs':m},m)['count'],1)
 def test_missing_input_denied(self):
  with self.assertRaises(ValueError):r.verify_inputs({'inputs':{'x':'a'*64}},{})
 def test_extra_input_denied(self):
  with self.assertRaises(ValueError):r.verify_inputs({'inputs':{'x':'a'*64}},{'x':'a'*64,'y':'b'*64})
 def test_hash_drift_denied(self):
  with self.assertRaises(ValueError):r.verify_inputs({'inputs':{'x':'a'*64}},{'x':'b'*64})
 def test_invalid_input_paths_denied(self):
  for k in ('/etc/passwd','../x','a/../../x'):
   with self.assertRaises(ValueError):r.verify_inputs({'inputs':{k:'a'*64}},{k:'a'*64})
 def test_invalid_input_hash_denied(self):
  with self.assertRaises(ValueError):r.verify_inputs({'inputs':{'x':'bad'}},{'x':'bad'})
 def test_same_package_exact(self):r.same_package({'header':{'id':'x'},'archive_sha256':'a'}, {'passed':True,'header':{'id':'x'},'archive_sha256':'a'})
 def test_package_member_drift_denied(self):
  with self.assertRaises(ValueError):r.same_package({'member_sha256':{'x':'a'}},{'passed':True,'member_sha256':{'x':'b'}})
 def test_package_archive_drift_denied(self):
  with self.assertRaises(ValueError):r.same_package({'archive_sha256':'a'},{'archive_sha256':'b'})
 def test_unknown_package_field_denied(self):
  with self.assertRaises(ValueError):r.same_package({'a':1,'extra':2},{'a':1})
 def test_compiler_wrappers_deny(self):
  with tempfile.TemporaryDirectory() as d:
   p=Path(d)/'denied';r.forbidden_tools(p)
   for n in ('cargo','rustc'):self.assertIn('sys.exit(97)',(p/n).read_text());self.assertNotIn('subprocess',(p/n).read_text())
 def test_no_plan_or_build_call(self):
  tree=ast.parse((HERE/'run.py').read_text());calls=[n for n in ast.walk(tree) if isinstance(n,ast.Call)]
  self.assertFalse(any(isinstance(x.func,ast.Attribute) and x.func.attr in ('attempt_plan','cli_witness') for x in calls));self.assertNotIn("'build'",(HERE/'run.py').read_text());self.assertNotIn("'apply'",(HERE/'run.py').read_text())
 def test_full_inputs_and_packages_before_guards(self):
  s=(HERE/'run.py').read_text();self.assertLess(s.index("report['gates']['official_inputs']=verify_inputs"),s.index('guards=prior.attempt_guards'));self.assertLess(s.index("report['gates']['full_plan_contract']=package.verify_plan"),s.index('guards=prior.attempt_guards'));self.assertIn("source/'plan.json',official,integration",s)
 def test_fixed_artifact_workflow(self):
  w=(r.ROOT/'.github/workflows/playtest-guard-replay-wip.yml').read_text();self.assertIn('actions/download-artifact@d3f86a106a0bac45b974a628896c90dbdf5c8093',w);self.assertIn("artifact-ids: '13513252348'",w);self.assertIn('run-id: 42236543685',w);self.assertLess(w.index('--preflight'),w.index('actions/download-artifact@'));self.assertIn('actions: read',w)
 def test_original_artifact_identity(self):
  a=r.inputs()['artifact'];self.assertEqual(a['source_head'],'cea160c079c8a3f008939fad5986046372d6f69f');self.assertEqual(set(a['files']),{'manifest.json','plan.json','receipt.json','package-maps.json'});self.assertEqual(a['files']['plan.json']['sha256'],'7139236d6484c9cf5a862d3e8358d60b2b4e0bd3a8c4c0968c6d313735670f8b')
if __name__=='__main__':unittest.main()
