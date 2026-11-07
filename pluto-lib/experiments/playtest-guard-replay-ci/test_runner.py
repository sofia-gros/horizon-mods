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
  w=(r.ROOT/'.github/workflows/playtest-guard-replay-wip.yml').read_text();self.assertIn('actions/download-artifact@d3f86a106a0bac45b974a628896c90dbdf5c8093',w);self.assertIn("artifact-ids: '13556955569'",w);self.assertIn('run-id: 42328100345',w);self.assertLess(w.index('--preflight'),w.index('actions/download-artifact@'));self.assertIn('actions: read',w)
 def test_original_artifact_identity(self):
  a=r.inputs()['artifact'];self.assertEqual(a['source_head'],'7082868bcbf37ed3eabd32f40ddf0cf08a4ee30c');self.assertEqual(set(a['files']),{'manifest.json','plan.json','receipt.json','package-maps.json'});self.assertEqual(a['files']['plan.json']['sha256'],'dc4fbc3c54144f2f95728986b4e9bfb3700235655b18a3af3c8d1a6113dcb86b')
 def package_pair(self):
  value={'id':'tconstruct_native','members':3,'header':{'id':'tconstruct_native'},'file_mode':'0644','archive_sha256':'a'*64,'archive_bytes':100,'unpacked_bytes':200,'member_sha256':{r.SUPPORT:'a'*64,'horizon-native.json':'h','mixins/tconstruct_native/x.rs':'r'},'support':{'members':1,'unpacked_bytes':2},'support_seal_sha256':'s'}
  old=copy.deepcopy(value);old['passed']=True
  plan={'data':[{'mod_id':'tconstruct_native','files':{'tconstruct-support.zip':'a'*64}}]}
  return value,old,plan
 def test_support_data_separated(self):
  actual,old,plan=self.package_pair();actual['member_sha256'][r.SUPPORT]='b'*64;actual['archive_sha256']='c'*64;actual['archive_bytes']=105;actual['unpacked_bytes']=205;actual['support_seal_sha256']='t'
  result=r.native_package_equality(actual,old,plan);self.assertTrue(result['native_members_equal']);self.assertFalse(result['archive_equal']);self.assertTrue(result['support_changed']);self.assertIn('No apply',result['scope'])
 def test_support_unchanged_is_explicit(self):
  actual,old,plan=self.package_pair();result=r.native_package_equality(actual,old,plan);self.assertTrue(result['archive_equal']);self.assertFalse(result['support_changed'])
 def test_native_member_changed_denied(self):
  actual,old,plan=self.package_pair();actual['member_sha256']['mixins/tconstruct_native/x.rs']='new'
  with self.assertRaises(ValueError):r.native_package_equality(actual,old,plan)
 def test_native_member_removed_denied(self):
  actual,old,plan=self.package_pair();del actual['member_sha256']['mixins/tconstruct_native/x.rs']
  with self.assertRaises(ValueError):r.native_package_equality(actual,old,plan)
 def test_header_changed_denied(self):
  actual,old,plan=self.package_pair();actual['header']['id']='changed'
  with self.assertRaises(ValueError):r.native_package_equality(actual,old,plan)
 def test_unknown_tc_field_denied(self):
  actual,old,plan=self.package_pair();actual['newfield']=True
  with self.assertRaises(ValueError):r.native_package_equality(actual,old,plan)
 def test_old_plan_data_drift_denied(self):
  actual,old,plan=self.package_pair();plan['data'][0]['files']['tconstruct-support.zip']='b'*64
  with self.assertRaises(ValueError):r.native_package_equality(actual,old,plan)
 def test_extra_plan_data_denied(self):
  actual,old,plan=self.package_pair();plan['data'].append({'mod_id':'extra','files':{}})
  with self.assertRaises(ValueError):r.native_package_equality(actual,old,plan)
 def test_combined_support_only_allowed(self):
  old={'headers':[{'id':'tconstruct_native'}],'members':2,'unpacked_bytes':2,'member_sha256':{r.COMBINED_SUPPORT:'a','x.rs':'same'},'members_seal_sha256':'before'};actual=copy.deepcopy(old);actual['member_sha256'][r.COMBINED_SUPPORT]='b';actual['members_seal_sha256']='after';actual['unpacked_bytes']=3
  self.assertEqual(r.native_combined_equality(actual,old)['native_members'],1)
  actual['member_sha256']['x.rs']='bad'
  with self.assertRaises(ValueError):r.native_combined_equality(actual,old)
 def test_contract_precedes_guards(self):
  text=(HERE/'run.py').read_text();self.assertLess(text.index("report['gates']['full_plan_contract']=package.verify_plan"),text.index('contract=prior.write_package_contract'));self.assertLess(text.index('contract=prior.write_package_contract'),text.index('guards=prior.attempt_guards'));self.assertIn("source/'plan.json',official,integration,contract_path",text);self.assertIn("'crafting_package'",text)
 def test_final_contract_archive_seals(self):
  text=(HERE/'run.py').read_text();self.assertIn("'Crafting contract changed'",text);self.assertIn("'Admitted package changed'",text)
 def test_ready_false_blocks_admission(self):
  c=r.inputs();c['ready']=False
  with self.assertRaisesRegex(ValueError,'not ready'):r.validate(c)
if __name__=='__main__':unittest.main()
