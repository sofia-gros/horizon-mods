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
  text=(HERE/'safety.py').read_text();tree=ast.parse(text)
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
if __name__=='__main__':unittest.main()
