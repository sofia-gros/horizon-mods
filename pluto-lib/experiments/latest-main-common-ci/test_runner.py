import ast,contextlib,hashlib,importlib.util,io,json,os,tempfile,unittest,zipfile
from pathlib import Path
from unittest import mock
HERE=Path(__file__).resolve().parent
s=importlib.util.spec_from_file_location('baseline_ci',HERE/'run.py');m=importlib.util.module_from_spec(s);s.loader.exec_module(m)
p=m.prepare
class Tests(unittest.TestCase):
 def test_01_exact_sources_and_pins(self):m.validate(m.inputs())
 def test_02_other_pins_rejected(self):
  for key,value in [('main','0'*40),('branch','other'),('common_bundle_sha256','0'*64)]:
   x=m.inputs();x[key]=value
   with self.assertRaises(ValueError):m.validate(x)
 def test_03_other_official_source_rejected(self):
  x=m.inputs();x['official']['horizon']['url']='https://example.invalid/fork'
  with self.assertRaises(ValueError):m.validate(x)
 def test_04_missing_input_rejected(self):
  x=m.inputs();x['source_sha256'].pop(next(iter(x['source_sha256'])))
  with self.assertRaises(ValueError):m.validate(x)
 def test_05_exact_public_event_only(self):
  e={'GITHUB_ACTIONS':'true','GITHUB_REPOSITORY':m.REPOSITORY,'GITHUB_EVENT_NAME':'push','GITHUB_REF':'refs/heads/'+m.BRANCH};v={'repository':{'private':False}}
  self.assertTrue(m.event_allowed(e,v));self.assertFalse(m.event_allowed({},v));self.assertFalse(m.event_allowed(e,{'repository':{'private':True}}))
 def test_06_metadata_exact_argv(self):
  path=Path('/official/Cargo.toml');a=p.metadata_args(path)
  self.assertTrue(p.allowed('cargo',a,path))
  for bad in [['build'],a[:-1],a+['--all-features'],[v.replace('/official','/other') for v in a]]:self.assertFalse(p.allowed('cargo',bad,path))
 def test_07_rustc_versions_only(self):
  for a in [['--version'],['-vV']]:self.assertTrue(p.allowed('rustc',a,Path('/x')))
  for a in [[],['--crate-name','horizon_client'],['--version','src/lib.rs']]:self.assertFalse(p.allowed('rustc',a,Path('/x')))
 def test_08_generated_wrapper_refuses_before_exec(self):
  import runpy,sys
  with tempfile.TemporaryDirectory() as d:
   root=Path(d);p.write_wrappers(root/'bin',Path('/official/Cargo.toml'),Path('/unused/cargo'),Path('/unused/rustc'),root/'argv')
   with mock.patch.object(sys,'argv',['cargo','build']),mock.patch('os.execv') as execute,contextlib.redirect_stderr(io.StringIO()):
    with self.assertRaises(SystemExit) as err:runpy.run_path(str(root/'bin/cargo'),run_name='__main__')
   self.assertEqual(err.exception.code,97);execute.assert_not_called();self.assertFalse(json.loads((root/'argv').read_text())['allowed'])
 def test_09_generated_wrapper_passes_exact_metadata(self):
  import runpy,sys
  with tempfile.TemporaryDirectory() as d:
   root=Path(d);manifest=Path('/official/Cargo.toml');p.write_wrappers(root/'bin',manifest,Path('/unused/cargo'),Path('/unused/rustc'),root/'argv')
   with mock.patch.object(sys,'argv',['cargo',*p.metadata_args(manifest)]),mock.patch('os.execv',side_effect=RuntimeError('stub-exec')) as execute:
    with self.assertRaisesRegex(RuntimeError,'stub-exec'):runpy.run_path(str(root/'bin/cargo'),run_name='__main__')
   execute.assert_called_once_with('/unused/cargo',['/unused/cargo',*p.metadata_args(manifest)])
 def test_10_complete_diagnostic_only(self):
  value={'code':'E_REVISION','message':'exact mismatch','file':'a'};self.assertEqual(p.diagnostic(json.dumps(value)),value)
  for bad in ['prefix '+json.dumps(value),json.dumps({'message':'missing code'}),json.dumps({**value,'ignored':True})]:
   with self.assertRaises((ValueError,json.JSONDecodeError)):p.diagnostic(bad)
 def test_11_first_rejection_retained_once(self):
  with tempfile.TemporaryDirectory() as d:
   work=Path(d);(work/'official-plan.log').write_text(json.dumps({'code':'E_CRATE','message':'before revision'}))
   runner=mock.Mock(work=work,report={'phases':[{'name':'official-plan','exit_code':1,'error':'RuntimeError: official-plan exited 1'}]})
   runner.run.side_effect=RuntimeError('official-plan exited 1')
   result=m.attempt_plan(runner,Path('/cli'),Path('/official'),Path('/mods'),work/'plan.json')
   self.assertFalse(result['accepted']);self.assertEqual(result['diagnostic']['code'],'E_CRATE');self.assertEqual(runner.run.call_count,1)
 def test_12_guard_failure_is_not_compatibility_result(self):
  runner=mock.Mock(report={'phases':[{'name':'official-plan','exit_code':-9,'error':'RuntimeError: Disk floor reached'}]})
  runner.run.side_effect=RuntimeError('Disk floor reached')
  with self.assertRaisesRegex(RuntimeError,'Disk floor'):m.attempt_plan(runner,Path('/c'),Path('/h'),Path('/m'),Path('/p'))
  self.assertEqual(runner.run.call_count,1)
 def test_13_plan_success_requires_plan_file(self):
  with tempfile.TemporaryDirectory() as d:
   path=Path(d)/'plan.json';runner=mock.Mock();runner.run.return_value='{"status":"ok"}'
   with self.assertRaises(ValueError):m.attempt_plan(runner,Path('/c'),Path('/h'),Path('/m'),path)
 def test_14_zip_duplicate_and_traversal_rejected(self):
  import warnings
  for names in [['a','a'],['../a'],['a/../b'],['/a'],['a\\b']]:
   b=io.BytesIO()
   with warnings.catch_warnings():
    warnings.simplefilter('ignore')
    with zipfile.ZipFile(b,'w') as z:
     for name in names:
      i=zipfile.ZipInfo(name);i.external_attr=0o100644<<16;z.writestr(i,b'x')
   b.seek(0)
   with self.assertRaises(ValueError):p.members(b)
 def test_15_changed_common_rejected_without_output(self):
  with tempfile.TemporaryDirectory() as d:
   root=Path(d);f=root/'wrong';f.write_bytes(b'not successful common')
   with self.assertRaises(ValueError):p.compose(root/'main',f,root/'out')
   self.assertFalse((root/'out').exists())
 def test_16_cli_actual_compiler_and_no_game(self):
  line='Running `rustc --crate-name horizon_mixins crates/horizon-mixins-cli/src/main.rs`'
  self.assertEqual(m.cli_witness(line)['crate'],'horizon_mixins')
  for log in ['',line+'\n'+line,line+'\nRunning `rustc --crate-name horizon_client src/main.rs`']:
   with self.assertRaises(ValueError):m.cli_witness(log)
 def test_17_memory_unknown_remains_blocked(self):
  with tempfile.TemporaryDirectory() as d:
   root=Path(d);(root/'self').mkdir();(root/'self/mountinfo').write_text('');(root/'self/cgroup').write_text('0::/\n');(root/'meminfo').write_text('MemTotal: 10000000 kB\nMemAvailable: 9000000 kB\n')
   with self.assertRaisesRegex(ValueError,'unambiguous cgroup2'):m.safety.check_memory(1,proc=root,root=root/'not-mounted')
 def test_18_memory_function_bytes_unchanged(self):
  text=(HERE.parent/'main-common-ci/safety.py').read_text();tree=ast.parse(text)
  expected=m.inputs()['memory_function_sha256']
  for name,digest in expected.items():
   node=next(n for n in tree.body if isinstance(n,ast.FunctionDef) and n.name==name)
   self.assertEqual(hashlib.sha256(ast.get_source_segment(text,node).encode()).hexdigest(),digest)
 def test_19_execution_stays_plan_only(self):
  text=(HERE/'run.py').read_text();tree=ast.parse(text)
  actions=[n.args[0].value for n in ast.walk(tree) if isinstance(n,ast.Call) and isinstance(n.func,ast.Attribute) and n.func.attr=='run' and n.args and isinstance(n.args[0],ast.Constant)]
  self.assertEqual(actions.count('official-plan'),1)
  for forbidden in ['official-apply','host-type','native32','native160','cargo-test']:self.assertNotIn(forbidden,actions)
  self.assertLess(text.index("witness=cli_witness"),text.index("env.update(CARGO="));self.assertIn("'compatibility':None",text)
 def test_20_helpers_pgid_and_receipt_contract(self):
  text=(m.ROOT/'pluto-lib/experiments/common-composite-ci/run.py').read_text()
  self.assertIn('start_new_session=True',text);self.assertIn('ensure_group_gone(proc.pid, name)',text)
  own=(HERE/'run.py').read_text();self.assertIn("'source_verified':False",own);self.assertIn("'probe_completed':False",own)
  self.assertIn("False if not result['accepted'] else None",own)


 def pair(self):
  result=[];payload={}
  for path,prior in [('tconstruct_native/horizon-mixin.toml','0c7b5b61a5e746d621afffba919b12aeccf9c0fa'),('pluto_native_creative/horizon-mixin.toml','624c4f6265c02296fbe8a80f029dd0d987fb94c0')]:
   before='horizon_revision = "'+prior+'"';after='horizon_revision = "'+p.HORIZON+'"'
   body=('[mod]\nid = "example"\n[compatibility]\n'+before+'\n').encode()
   result.append(dict(path=path,prior=prior,before_line=before,after_line=after,bytes=len(body),before_sha256=p.sha(body),after_sha256=p.sha(body.replace(before.encode(),after.encode()))));payload[path]=body
  for i in range(642):payload['tconstruct_native/body'+str(i)+'.rs']=b'unchanged body'
  return result,payload
 def test_21_revision_only_semantics_and_inverse(self):
  items,payload=self.pair()
  for item in items:
   body=payload[item['path']];changed=p.revised_manifest(body,item)
   self.assertEqual(changed.replace(item['after_line'].encode(),item['before_line'].encode()),body)
   self.assertEqual(changed.count(p.HORIZON.encode()),1)
 def test_22_wrong_original_or_candidate_hash_rejected(self):
  items,payload=self.pair();item=items[0];body=payload[item['path']]
  with self.assertRaises(ValueError):p.revised_manifest(body+b'\n',item)
  with self.assertRaises(ValueError):p.revised_manifest(body,{**item,'after_sha256':'0'*64})
 def test_23_full_644_member_inverse(self):
  items,payload=self.pair()
  with tempfile.TemporaryDirectory() as d:
   root=Path(d);original=root/'old';original.mkdir()
   for key,body in payload.items():
    path=original/key;path.parent.mkdir(parents=True,exist_ok=True);path.write_bytes(body)
   with mock.patch.object(p,'contract',return_value={'manifest_changes':items}):
    evidence=p.revise(original,root/'candidate',{k:p.sha(b) for k,b in payload.items()})
   self.assertEqual(evidence['change_count'],2);self.assertEqual(evidence['unchanged_members'],642);self.assertTrue(evidence['inverse_matches_original'])
   self.assertEqual({key for key in payload if (root/'candidate'/key).read_bytes()!=payload[key]},set(i['path'] for i in items))
   self.assertTrue(all((original/key).read_bytes()==body for key,body in payload.items()))
 def test_24_original_drift_refused_before_output(self):
  items,payload=self.pair()
  with tempfile.TemporaryDirectory() as d:
   root=Path(d);original=root/'old';original.mkdir();(original/'extra').write_bytes(b'other')
   with mock.patch.object(p,'contract',return_value={'manifest_changes':items}),self.assertRaises(ValueError):p.revise(original,root/'candidate',{})
   self.assertFalse((root/'candidate').exists())
 def test_25_original_archive_and_raw_headers_not_rewritten(self):
  text=(HERE/'run.py').read_text()
  self.assertLess(text.index('prepare.verify_main_bundle'),text.index('prepare.revise'))
  self.assertLess(text.index("raise ValueError('Common archive differs"),text.index('prepare.revise'))
  self.assertIn("'headers':evidence['headers']",text)
  self.assertEqual(p.MAIN_BUNDLE,'8178b440266664d3db4dd9784af02894ebdb29910b0dd8f13504b2d341cf8f42')
 def test_26_guard_rejection_preserved_without_retry(self):
  with tempfile.TemporaryDirectory() as d:
   work=Path(d);(work/'complete-guards.log').write_text('Pluto native preflight rejected: exact old body changed\n')
   runner=mock.Mock(work=work,report={'phases':[{'name':'complete-guards','exit_code':1,'error':'RuntimeError: complete-guards exited 1'}]});runner.run.side_effect=RuntimeError('complete-guards exited 1')
   with mock.patch.object(m.common,'verify_plan',return_value={}):result=m.attempt_guards(runner,Path('/p'),Path('/h'))
   self.assertFalse(result['passed']);self.assertTrue(result['aggregate_invoked']);self.assertEqual(runner.run.call_count,1)
   runner.reset_mock();runner.report={'phases':[{'name':'complete-guards','exit_code':-9,'error':'memory floor'}]}
   with mock.patch.object(m.common,'verify_plan',return_value={}),self.assertRaises(RuntimeError):m.attempt_guards(runner,Path('/p'),Path('/h'))
 def test_27_payload_failure_and_guard_success_distinct(self):
  runner=mock.Mock()
  with mock.patch.object(m.common,'verify_plan',side_effect=ValueError('wrong owner')):result=m.attempt_guards(runner,Path('/p'),Path('/h'))
  self.assertFalse(result['passed']);self.assertFalse(result['aggregate_invoked']);runner.run.assert_not_called()
  with mock.patch.object(m.common,'verify_plan',return_value={}):result=m.attempt_guards(runner,Path('/p'),Path('/h'))
  self.assertTrue(result['passed']);self.assertEqual(result['groups_configured'],12);runner.run.assert_called_once()
 def test_28_latest_marker_and_prior_source_unchanged(self):
  with tempfile.TemporaryDirectory() as d:
   root=Path(d);destination=root/'out'
   def exported(*args):destination.mkdir();(destination/'official').write_bytes(b'exact')
   with mock.patch.object(m.common,'export',side_effect=exported):m.export_latest(root/'git',p.HORIZON,destination)
   self.assertEqual((destination/'.horizon-revision').read_text(),p.HORIZON+'\n')
   with mock.patch.object(m.common,'export') as export,self.assertRaises(ValueError):m.export_latest(root/'git','0'*40,root/'other')
   export.assert_not_called()

if __name__=='__main__':unittest.main()
