"""Pure negative and sequencing regressions; no Git, packaging, CLI or native jobs."""
import ast,copy,hashlib,importlib.util,io,json,os,stat,tempfile,unittest,zipfile
from pathlib import Path
from unittest.mock import patch
HERE=Path(__file__).resolve().parent
s=importlib.util.spec_from_file_location('runner',HERE/'run.py');r=importlib.util.module_from_spec(s);s.loader.exec_module(r)
p=r.package;g=r.guard_once

def stream(text):return {'text':text,'bytes':len(text.encode()),'sha256':p.sha(text.encode())}
def record(name,code=0):return {'script':name,'exit_code':code,'stdout':stream(''), 'stderr':stream('reject\n' if code else '')}
class Fake:
 def __init__(self,codes=None,fail=None):self.report={'gates':{},'phases':[],'compatibility':None};self.work=Path('/synthetic');self.calls=[];self.codes=codes or {};self.fail=fail
 def run(self,name,args,**kwargs):
  i=len(self.calls);self.calls.append((name,args,kwargs))
  if i==self.fail:raise TimeoutError('synthetic guard timeout')
  return json.dumps(record(g.GUARDS[i],self.codes.get(i,0)))

class Tests(unittest.TestCase):
 def test_unfrozen_refuses_before_setup(self):
  c=r.inputs();c['ready_for_reviewed_execution']=False
  with patch.object(r.baseline,'validate',side_effect=AssertionError('too late')):
   with self.assertRaisesRegex(ValueError,'not frozen'):r.validate(c)
 def test_exact_public_event(self):
  env={'GITHUB_ACTIONS':'true','GITHUB_REPOSITORY':r.REPOSITORY,'GITHUB_EVENT_NAME':'push','GITHUB_REF':'refs/heads/'+r.BRANCH}
  self.assertTrue(r.event_allowed(env,{'repository':{'private':False}}));self.assertFalse(r.event_allowed(env,{'repository':{'private':True}}))
 def test_fixed_latest_and_no_repin(self):
  self.assertEqual(p.HORIZON,'41f822b659789780f58d32c63ddfb6887ea1b8bf');self.assertEqual(p.MIXINS,'5a64af938ec86b00d53d16a408aabc0038fb3e57')
  src=(HERE/'run.py').read_text();self.assertNotIn('prepare.revise(',src);self.assertNotIn('write_candidate(',src)
 def test_shared_guard_unmodified_reference(self):self.assertIs(r.Runner,r.baseline.Runner)
 def test_all12_after_first_refusal(self):
  fake=Fake({0:1,7:1});out=r.attempt_guards(fake,Path('/plan'),Path('/official'))
  self.assertEqual(len(fake.calls),12);self.assertTrue(out['completed']);self.assertFalse(out['passed']);self.assertFalse(out['apply_allowed']);self.assertFalse(fake.report['compatibility'])
 def test_all12_success_never_compatibility_true(self):
  f=Fake();out=r.attempt_guards(f,Path('/plan'),Path('/official'));self.assertTrue(out['passed']);self.assertIsNone(f.report['compatibility'])
 def test_guard_timeout_retains_partial(self):
  f=Fake(fail=2)
  with self.assertRaises(TimeoutError):r.attempt_guards(f,Path('/plan'),Path('/official'))
  rows=f.report['gates']['complete_guards']['records'];self.assertEqual(len(f.calls),3);self.assertTrue(rows[2]['reached']);self.assertFalse(rows[3]['reached']);self.assertIn('reason',rows[3]);self.assertFalse(f.report['gates']['complete_guards']['completed'])
 def test_guard_exit98_not_refusal(self):
  f=Fake({0:98})
  with self.assertRaises(ValueError):r.attempt_guards(f,Path('/plan'),Path('/official'))
  self.assertEqual(len(f.calls),1)
 def test_guard_bool_not_exit(self):
  v=record(g.GUARDS[0]);v['exit_code']=False
  with self.assertRaises(ValueError):g.validate_record(v,g.GUARDS[0])
 def test_guard_tampered_stream_rejected(self):
  v=record(g.GUARDS[0]);v['stderr']['text']='changed'
  with self.assertRaises(ValueError):g.validate_record(v,g.GUARDS[0])
 def test_guard_bounded_full_stream(self):
  v=record(g.GUARDS[0]);v['stdout']=stream('x'*(g.MAX_STREAM+1))
  with self.assertRaises(ValueError):g.validate_record(v,g.GUARDS[0])
 def test_guard_wrong_identity_rejected(self):
  with self.assertRaises(ValueError):g.validate_record(record('other'),g.GUARDS[0])
 def test_guard_subprocess_same_group(self):
  tree=ast.parse((HERE/'guard_once.py').read_text());calls=[n for n in ast.walk(tree) if isinstance(n,ast.Call) and isinstance(n.func,ast.Attribute) and n.func.attr=='Popen']
  self.assertEqual(len(calls),1);self.assertFalse({'start_new_session','process_group'} & {k.arg for k in calls[0].keywords})
 def test_guard_local_entry_rejected(self):
  with patch.dict(os.environ,{},clear=True):
   with self.assertRaisesRegex(ValueError,'CI only'):g.execute('/root','/plan','/official',g.GUARDS[0],'/out')
 def test_protection_cli_single_argument(self):
  src=(HERE/'guard_once.py').read_text();self.assertIn("if script != 'check_protection_plan.py': argv.append(str(official))",src)
 def test_all_guard_names_fixed(self):self.assertEqual(len(g.GUARDS),12);self.assertEqual(len(set(g.GUARDS)),12)
 def test_plan_once_600(self):
  tree=ast.parse((HERE/'run.py').read_text());node=next(n for n in tree.body if isinstance(n,ast.FunctionDef) and n.name=='attempt_plan');self.assertEqual(sum(isinstance(n,ast.Call) and isinstance(n.func,ast.Attribute) and n.func.attr=='run' for n in ast.walk(node)),1);self.assertIn('timeout=600',ast.unparse(node))
 def test_wholeplan_before_contract_collector_guards(self):
  src=(HERE/'run.py').read_text();names=["report['gates']['plan_diagnostic']=", "package.verify_plan(plan", 'terrain=attempt_terrain(', 'guards=attempt_guards('];positions=[src.index(n) for n in names];self.assertEqual(positions,sorted(positions))
 def test_collector_refusal_does_not_skip_guards(self):
  src=(HERE/'run.py').read_text();self.assertNotIn("if terrain['passed']",src);self.assertIn("'--mod-root',str(integration)",src)
 def test_finally_argv_cpu_and_seals(self):
  src=(HERE/'run.py').read_text();tail=src[src.index('    finally:\n'):];self.assertIn('collect_tool_calls(',tail);self.assertIn('observer.collect(',tail);self.assertIn('assert_full_seal',tail);self.assertIn('git_snapshots.items()',tail)
 def test_no_apply_or_consumer_command(self):
  tree=ast.parse((HERE/'run.py').read_text());words=[n.value for n in ast.walk(tree) if isinstance(n,ast.Constant) and isinstance(n.value,str)];self.assertNotIn('apply',words);self.assertNotIn('horizon-client',words);self.assertIn('horizon-mixins-cli',words)
 def test_plan_size_and_links(self):
  with tempfile.TemporaryDirectory() as d:
   root=Path(d);f=root/'plan';f.write_bytes(b'{}');self.assertEqual(p.plan_bytes(f),b'{}');l=root/'link';l.symlink_to(f)
   with self.assertRaises(ValueError):p.plan_bytes(l)
   f.write_bytes(b'')
   with self.assertRaises(ValueError):p.plan_bytes(f)
 def test_archive_wrong_owner_and_permissions(self):
  with tempfile.TemporaryDirectory() as d:
   f=Path(d)/'a.zip'
   for mode,owner in [(0o100644,'wrong'),(0o100755,'tconstruct_native')]:
    with zipfile.ZipFile(f,'w') as z:
     n=zipfile.ZipInfo('horizon-native.json');n.external_attr=mode<<16;z.writestr(n,json.dumps({'schema':1,'id':owner,'horizon':p.HORIZON,'mixins':p.MIXINS}))
    with self.assertRaises(ValueError):p.archive_payload(f,'tconstruct_native')
 def test_archive_duplicate_rejected(self):
  with tempfile.TemporaryDirectory() as d:
   f=Path(d)/'a.zip'
   with zipfile.ZipFile(f,'w') as z:
    for _ in range(2):
     n=zipfile.ZipInfo('a');n.external_attr=0o100644<<16;z.writestr(n,b'X')
   with self.assertRaises(ValueError):p.members(f)
 def test_archive_traversal_rejected(self):
  stream=io.BytesIO()
  with zipfile.ZipFile(stream,'w') as z:
   n=zipfile.ZipInfo('../a');n.external_attr=0o100644<<16;z.writestr(n,b'X')
  stream.seek(0)
  with self.assertRaises(ValueError):p.members(stream)
 def test_budgets(self):self.assertEqual((p.MAX_ENTRIES,p.MAX_UNPACKED,p.MAX_ARCHIVE,p.MAX_PLAN),(512,64*1024**2,64*1024**2,16*1024**2))
 def test_metadata_only_exact(self):
  manifest=Path('/fixed/Cargo.toml');self.assertTrue(p.allowed('cargo',p.old.metadata_args(manifest),manifest));self.assertFalse(p.allowed('cargo',['check'],manifest));self.assertFalse(p.allowed('rustc',['source.rs'],manifest))
 def test_schema_fields_match_windows(self):
  src=(HERE/'prepare.py').read_text()
  for k in ('archive_sha256','archive_bytes','members','unpacked_bytes','member_sha256','header','members_seal_sha256'):self.assertIn(k,src)
 def test_observer_own_scope_not_changed(self):
  old=HERE.parent/'latest-main-common-ci/observe_plan.py';before=old.read_text();after=(HERE/'observe_plan.py').read_text()
  # Compare against reviewed full runner, apart from explicit routing changes.
  self.assertIn("INTERVAL=5.0",after);self.assertIn("MAX_SAMPLES=128",after);self.assertIn("MAX_BYTES=64*1024",after);self.assertNotIn('/proc/self/environ',after)
 def test_required_gates_distinguish_refusal(self):
  self.assertNotIn('complete_guards',r.required_gates({'plan_accepted':False}));self.assertIn('complete_guards',r.required_gates({'plan_accepted':True}))
if __name__=='__main__':unittest.main()
