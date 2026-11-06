"""Pure negative and sequencing regressions; no Git, packaging, CLI or native jobs."""
import ast,contextlib,copy,hashlib,importlib.util,io,json,os,stat,tempfile,unittest,warnings,zipfile
from pathlib import Path
from unittest.mock import patch
HERE=Path(__file__).resolve().parent
s=importlib.util.spec_from_file_location('runner',HERE/'run.py');r=importlib.util.module_from_spec(s);s.loader.exec_module(r)
p=r.package;g=r.guard_once

def stream(text):return {'text':text,'bytes':len(text.encode()),'sha256':p.sha(text.encode())}
def record(name,code=0):return {'script':name,'exit_code':code,'stdout':stream(''), 'stderr':stream(g.REJECTION_PREFIX.get(name,'')+'synthetic refusal\n' if code else '')}
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
  fake=Fake({0:1,7:1});out=r.attempt_guards(fake,Path('/plan'),Path('/official'),Path('/fixed-integration'))
  self.assertEqual(len(fake.calls),12);self.assertTrue(out['completed']);self.assertFalse(out['passed']);self.assertFalse(out['apply_allowed']);self.assertFalse(fake.report['compatibility'])
 def test_all12_success_never_compatibility_true(self):
  f=Fake();out=r.attempt_guards(f,Path('/plan'),Path('/official'),Path('/fixed-integration'));self.assertTrue(out['passed']);self.assertIsNone(f.report['compatibility'])
 def test_guard_timeout_retains_partial(self):
  f=Fake(fail=2)
  with self.assertRaises(TimeoutError):r.attempt_guards(f,Path('/plan'),Path('/official'),Path('/fixed-integration'))
  rows=f.report['gates']['complete_guards']['records'];self.assertEqual(len(f.calls),3);self.assertIsNone(rows[2]['reached']);self.assertTrue(rows[2]['attempted']);self.assertFalse(rows[3]['reached']);self.assertIn('reason',rows[3]);self.assertFalse(f.report['gates']['complete_guards']['completed'])
 def test_guard_exit98_not_refusal(self):
  f=Fake({0:98})
  with self.assertRaises(ValueError):r.attempt_guards(f,Path('/plan'),Path('/official'),Path('/fixed-integration'))
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
   with warnings.catch_warnings(record=True) as caught:
    warnings.simplefilter('always',UserWarning)
    with zipfile.ZipFile(f,'w') as z:
     for _ in range(2):
      n=zipfile.ZipInfo('a');n.external_attr=0o100644<<16;z.writestr(n,b'X')
   self.assertEqual([(w.category,str(w.message)) for w in caught],[(UserWarning,"Duplicate name: 'a'")])
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
 def test_guard_integration_root_not_controller(self):
  f=Fake();r.attempt_guards(f,Path('/plan'),Path('/official'),Path('/fixed-integration'))
  for _,argv,_ in f.calls:self.assertEqual(argv[2],'/fixed-integration');self.assertNotEqual(argv[2],str(r.ROOT))
 def test_unknown_exit1_is_transport_failure(self):
  v=record(g.GUARDS[0],1);v['stderr']=stream('Traceback (most recent call last):\n')
  with self.assertRaisesRegex(ValueError,'recognized refusal'):g.validate_record(v,g.GUARDS[0])
 def test_guard_prefix_is_script_specific(self):
  v=record(g.GUARDS[0],1);v['stderr']=stream(g.REJECTION_PREFIX[g.GUARDS[1]]+'failure\n')
  with self.assertRaises(ValueError):g.validate_record(v,g.GUARDS[0])
 def test_real_verbose_warning_case_passes_shared_validator(self):
  output=io.StringIO()
  with contextlib.redirect_stdout(output),contextlib.redirect_stderr(output):
   outcome=unittest.TextTestRunner(stream=output,verbosity=2).run(unittest.TestSuite([Tests('test_archive_duplicate_rejected')]))
  self.assertTrue(outcome.wasSuccessful());r.common.validate_python_checks(output.getvalue(),1)
class IndexControllerTests(unittest.TestCase):
 def core_line(self,path='crates/horizon-mixins-core/src/lib.rs'):
  return 'Running `rustc --crate-name horizon_mixins_core '+path+'`'
 def test_actual_core_witness(self):
  self.assertEqual(r.core_compile_witness(self.core_line(),Path('/mixins'))['crate'],'horizon_mixins_core')
 def test_core_wrong_source(self):
  with self.assertRaises(ValueError):r.core_compile_witness(self.core_line('wrong.rs'),Path('/mixins'))
 def test_core_missing_or_duplicate(self):
  for log in ['',self.core_line()+'\n'+self.core_line()]:
   with self.assertRaises(ValueError):r.core_compile_witness(log,Path('/mixins'))
 def test_actual_harness_witness(self):
  with tempfile.TemporaryDirectory() as d:
   root=Path(d);(root/'src').mkdir();(root/'src/main.rs').write_text('fn main(){}')
   value=r.index_compile_witness('Running `rustc --crate-name pluto_packet_index_observer src/main.rs`',root,Path('/mixins'))
   self.assertEqual(value['source_sha256'],p.sha(b'fn main(){}'))
 def test_harness_wrong_source_and_test(self):
  for tail in ['wrong.rs','src/main.rs --test']:
   with self.assertRaises(ValueError):r.index_compile_witness('Running `rustc --crate-name pluto_packet_index_observer '+tail+'`',Path('/fixture'),Path('/mixins'))
 def test_harness_consumer_rejected(self):
  with self.assertRaises(ValueError):r.index_compile_witness('Running `rustc --crate-name horizon_client src/lib.rs`',Path('/fixture'),Path('/mixins'))
 def test_index_metadata_same_group(self):
  tree=ast.parse((HERE/'run.py').read_text());fn=next(n for n in tree.body if isinstance(n,ast.FunctionDef) and n.name=='capture_index_metadata')
  calls=[n for n in ast.walk(fn) if isinstance(n,ast.Call) and isinstance(n.func,ast.Attribute) and n.func.attr=='Popen']
  self.assertEqual(len(calls),1);self.assertFalse({'start_new_session','process_group','shell'} & {k.arg for k in calls[0].keywords});self.assertIn('stdout',{k.arg for k in calls[0].keywords})
 def test_index_after_main_results(self):
  src=(HERE/'run.py').read_text();self.assertLess(src.index('result=attempt_plan('),src.index("index_output=runner.run('index-observation'"));self.assertLess(src.index('guards=attempt_guards('),src.index("index_output=runner.run('index-observation'"));self.assertIn('before_index_calls=len(calls)',src)
 def test_index_compile_after_primary_diagnostic(self):
  src=(HERE/'run.py').read_text();self.assertLess(src.index("report['diagnostic_complete']=True"),src.index("index_log=runner.run('index-build'"));self.assertIn("'--target-dir',str(work/'cli-target')",src)
 def test_index_metadata_local_denied(self):
  with patch.dict(os.environ,{},clear=True):
   with self.assertRaises(ValueError):r.capture_index_metadata('/cargo','/fixture','adapt')
 def test_index_bounds_and_final_preservation(self):
  src=(HERE/'run.py').read_text();self.assertIn("output.open('xb')",src);self.assertIn('8*1024**2',src);self.assertIn("[str(index_binary),str(official)],timeout=180",src);self.assertIn('evidence.persist(report',src)

class AuxiliaryReceiptTests(unittest.TestCase):
 def test_auxiliary_receipt_separate(self):
  src=(HERE/'run.py').read_text();self.assertIn("aux=report['index_receipt']",src);self.assertIn('common.record_error(aux,exc)',src);self.assertIn("not report['index_receipt']['completed']",src)
 def test_compiler_environment_restored(self):
  src=(HERE/'run.py').read_text();self.assertIn('runner.env=build_env',src);self.assertGreaterEqual(src.count('runner.env=env'),2);self.assertLess(src.index('runner.env=env'),src.index("index_output=runner.run('index-observation'"))

class EvidenceTests(unittest.TestCase):
 def report(self):return {'head':'a'*40,'gates':{},'error':'synthetic failure','probe_completed':False,'source_verified':False,'plan_accepted':None}
 def setup_plan(self,base,body=b'{"synthetic":true}'):
  f,_=r.evidence.paths(base,'a'*40);f.parent.mkdir();f.write_bytes(body);return f
 def test_partial_receipt_survives_failure(self):
  with tempfile.TemporaryDirectory() as d:
   report=self.report();out,m=r.evidence.persist(report,Path(d),lambda n:None);self.assertFalse(m['plan_present']);self.assertEqual(json.loads((out/'receipt.json').read_text()),report);r.evidence.verify_saved(out)
 def test_full_plan_roundtrip(self):
  with tempfile.TemporaryDirectory() as d:
   f=self.setup_plan(Path(d));report=self.report();b=f.read_bytes();report.update(plan_accepted=True);report['gates']['plan_diagnostic']={'accepted':True,'plan_bytes':len(b),'plan_sha256':p.sha(b)}
   out,m=r.evidence.persist(report,Path(d),lambda n:None);self.assertEqual((out/'plan.json').read_bytes(),b);self.assertTrue(report['artifact_preservation']['plan_matches_accepted_receipt']);r.evidence.verify_saved(out)
 def test_partial_plan_not_claimed_accepted(self):
  with tempfile.TemporaryDirectory() as d:
   self.setup_plan(Path(d),b'{incomplete');report=self.report();out,m=r.evidence.persist(report,Path(d),lambda n:None);self.assertTrue(m['plan_present']);self.assertFalse(report['artifact_preservation']['plan_matches_accepted_receipt'])
 def test_missing_accepted_plan_rejected(self):
  with tempfile.TemporaryDirectory() as d:
   report=self.report();report['plan_accepted']=True
   with self.assertRaisesRegex(ValueError,'missing'):r.evidence.persist(report,Path(d),lambda n:None)
 def test_plan_digest_drift_rejected(self):
  with tempfile.TemporaryDirectory() as d:
   self.setup_plan(Path(d));report=self.report();report['gates']['plan_diagnostic']={'accepted':True,'plan_bytes':1,'plan_sha256':'f'*64}
   with self.assertRaisesRegex(ValueError,'differs'):r.evidence.persist(report,Path(d),lambda n:None)
 def test_symlink_input_rejected(self):
  with tempfile.TemporaryDirectory() as d:
   f,_=r.evidence.paths(Path(d),'a'*40);f.parent.mkdir();target=Path(d)/'other';target.write_text('x');f.symlink_to(target)
   with self.assertRaisesRegex(ValueError,'symlink'):r.evidence.persist(self.report(),Path(d),lambda n:None)
 def test_hardlink_input_rejected(self):
  with tempfile.TemporaryDirectory() as d:
   f=self.setup_plan(Path(d));os.link(f,Path(d)/'other')
   with self.assertRaisesRegex(ValueError,'regular'):r.evidence.persist(self.report(),Path(d),lambda n:None)
 def test_oversized_plan_rejected(self):
  with tempfile.TemporaryDirectory() as d:
   f=self.setup_plan(Path(d));f.write_bytes(b'x'*40)
   with patch.dict(r.evidence.LIMITS,{'plan.json':32}):
    with self.assertRaises(ValueError):r.evidence.persist(self.report(),Path(d),lambda n:None)
 def test_existing_output_rejected(self):
  with tempfile.TemporaryDirectory() as d:
   _,out=r.evidence.paths(Path(d),'a'*40);out.mkdir()
   with self.assertRaisesRegex(ValueError,'already exists'):r.evidence.persist(self.report(),Path(d),lambda n:None)
 def test_guard_failure_no_ready_directory(self):
  with tempfile.TemporaryDirectory() as d:
   _,out=r.evidence.paths(Path(d),'a'*40)
   with self.assertRaises(RuntimeError):r.evidence.persist(self.report(),Path(d),lambda n:(_ for _ in ()).throw(RuntimeError('floor')))
   self.assertFalse(out.exists())
 def test_saved_tamper_or_extra_rejected(self):
  with tempfile.TemporaryDirectory() as d:
   out,_=r.evidence.persist(self.report(),Path(d),lambda n:None);(out/'receipt.json').write_text('{}')
   with self.assertRaises(ValueError):r.evidence.verify_saved(out)
   (out/'secret.env').write_text('synthetic')
   with self.assertRaises(ValueError):r.evidence.verify_saved(out)
 def test_fixed_upload_after_ready_only(self):
  w=(r.ROOT/'.github/workflows/playtest-complete-plan-wip.yml').read_text();self.assertIn('actions/upload-artifact@ea165f8d65b6e75b540449e92b4886f43607fa02',w);self.assertIn("always() && steps.diagnostic.outputs.evidence_ready == 'true'",w);self.assertIn('retention-days: 3',w)
  code=(HERE/'run.py').read_text();self.assertLess(code.index('evidence.verify_saved(output)'),code.index("out.write('evidence_ready=true"));self.assertIn('contents: read',w)

 def test_manifest_only_cannot_pass(self):
  with tempfile.TemporaryDirectory() as d:
   pth=Path(d);(pth/'manifest.json').write_text(json.dumps({'schema':1,'head':'a'*40,'plan_present':False,'source_verified':False,'probe_completed':False,'files':{}}))
   with self.assertRaisesRegex(ValueError,'file set'):r.evidence.verify_saved(pth)
 def test_manifest_types_strict(self):
  with tempfile.TemporaryDirectory() as d:
   out,_=r.evidence.persist(self.report(),Path(d),lambda n:None);path=out/'manifest.json';base=json.loads(path.read_text())
   for key,value in [('schema',True),('head','../bad'),('plan_present',0),('files',[])]:
    changed=copy.deepcopy(base);changed[key]=value;path.write_text(json.dumps(changed))
    with self.assertRaises(ValueError):r.evidence.verify_saved(out)
 def test_plan_presence_must_match_manifest(self):
  with tempfile.TemporaryDirectory() as d:
   out,m=r.evidence.persist(self.report(),Path(d),lambda n:None);m['plan_present']=True;(out/'manifest.json').write_text(json.dumps(m))
   with self.assertRaisesRegex(ValueError,'file set'):r.evidence.verify_saved(out)
 def test_receipt_identity_must_match_manifest(self):
  with tempfile.TemporaryDirectory() as d:
   out,m=r.evidence.persist(self.report(),Path(d),lambda n:None);m['head']='b'*40;(out/'manifest.json').write_text(json.dumps(m))
   with self.assertRaisesRegex(ValueError,'identity'):r.evidence.verify_saved(out)
 def test_accepted_plan_must_match_receipt(self):
  with tempfile.TemporaryDirectory() as d:
   out,m=r.evidence.persist(self.report(),Path(d),lambda n:None);rr=json.loads((out/'receipt.json').read_text());rr['plan_accepted']=True;rr['gates']['plan_diagnostic']={'accepted':True};b=r.evidence.encode(rr);(out/'receipt.json').write_bytes(b);m['files']['receipt.json']={'bytes':len(b),'sha256':p.sha(b)};(out/'manifest.json').write_text(json.dumps(m))
   with self.assertRaisesRegex(ValueError,'accepted plan'):r.evidence.verify_saved(out)

if __name__=='__main__':unittest.main()
