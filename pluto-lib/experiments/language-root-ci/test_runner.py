"""Pure synthetic controller/IR regressions. No planner, Cargo, Rust or official writes."""
import ast,contextlib,copy,importlib.util,io,json,os,tempfile,tomllib,unittest
from pathlib import Path
from unittest import mock
HERE=Path(__file__).resolve().parent
spec=importlib.util.spec_from_file_location('language_ci',HERE/'run.py');m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m)
p=m.probe

def payload():
    return {p.MOD+'/'+n:(p.candidate.HERE/n).read_bytes() for n in ['horizon-mixin.toml','root-store.rs']}

def local(name,ident):
    return {'id':ident,'name':name,'ordinal':0,'parameter':False,'mutable':False,'supported':True,'ty':None,'fingerprint':'a'*64}

def synthetic():
    files=payload();item=tomllib.loads(files[p.MOD+'/horizon-mixin.toml'].decode())['injection'][0]
    row={'mod_id':p.MOD,'mod_version':'0.0.0','injection_id':item['id'],'target':item['target'],'kind':item['kind'],
      'bindings':item['bindings'],'priority':100,'file':p.candidate.TARGET,'helper_source':files[p.MOD+'/root-store.rs'].decode(),
      'cancellable':False,'node':[1,2],'function_fingerprint':'f'*64,'ordinal':0,'require':1,
      'variable':{'at':'STORE','local':local('out',5)},'variable_site':3,'expression_site':3,'site_fingerprint':'e'*64,
      'contract':{'output':'std::path::PathBuf','value':'std::path::PathBuf'},
      'captures':[{'binding':'stamp','expression':'&stamp','ty':'&String','local':local('stamp',3)}]}
    return {'schema':6,'transformer':'synthetic-only','rustc':'synthetic-only','inputs':p.native_inputs(),'injections':[row],'modules':[]}

def verify(value):
    with tempfile.TemporaryDirectory() as tmp,mock.patch.object(p,'payload',return_value=payload()):
        path=Path(tmp)/'plan.json';path.write_text(json.dumps(value));return p.verify_plan(path,Path('/never-opened-official'),Path('/never-opened-mixins'))

class Tests(unittest.TestCase):
 def test_01_frozen_source_and_official_allowlist(self):
    m.validate(m.inputs())
    for key,value in [('repository','wrong'),('branch','wrong'),('candidate_source','0'*40)]:
        c=m.inputs();c[key]=value
        with self.assertRaises(ValueError):m.validate(c)
    for kind in ['horizon','mixins']:
        c=m.inputs();c['official'][kind]['url']='https://invalid.example/no'
        with self.assertRaises(ValueError):m.validate(c)
 def test_02_exact_file_set(self):
    c=m.inputs();c['source_sha256'].pop(next(iter(c['source_sha256'])))
    with self.assertRaises(ValueError):m.validate(c)
 def test_03_public_dedicated_trigger_only(self):
    env={'GITHUB_ACTIONS':'true','GITHUB_REPOSITORY':m.REPOSITORY,'GITHUB_EVENT_NAME':'push','GITHUB_REF':'refs/heads/'+m.BRANCH}
    self.assertTrue(m.event_allowed(env,{'repository':{'private':False}}))
    self.assertFalse(m.event_allowed(env,{'repository':{'private':True}}));self.assertFalse(m.event_allowed({},{}))
 def test_04_exact_controller_literal_derivation(self):
    before=(HERE.parent/'language-placement-ci/run.py').read_text();d=json.loads((HERE/'derivation.json').read_bytes())
    self.assertEqual(m.sha(before.encode()),d['source_sha256'])
    for item in d['changes']:
        self.assertEqual(before.count(item['before']),item['count']);before=before.replace(item['before'],item['after'])
    self.assertEqual(before,(HERE/'run.py').read_text())
 def test_05_observer_changes_only_branch_and_owned_directory(self):
    before=(HERE.parent/'language-placement-ci/observe_plan.py').read_text()
    after=(HERE/'observe_plan.py').read_text()
    self.assertEqual(after.replace(m.BRANCH,'checkpoint/plutolib-language-placement-ci-20261106').replace("work/'language-root-probe'","work/'language-placement-probe'"),before)
    self.assertEqual((m.observer.MAX_BYTES,m.observer.MAX_SAMPLES,m.observer.INTERVAL),(65536,128,5.0))
 def test_06_fresh_exact_two_file_output(self):
    files=payload()
    with tempfile.TemporaryDirectory() as tmp,mock.patch.object(p,'payload',return_value=files):
        root=Path(tmp)/'probe';result=p.write_probe(root,Path('/unused'),Path('/unused'))
        self.assertEqual(result['members'],2);self.assertEqual(result['injections'],1);self.assertEqual(result['modules'],0)
        self.assertEqual({x.relative_to(root).as_posix():x.read_bytes() for x in (root/p.MOD).iterdir()},files)
        with self.assertRaises(ValueError):p.write_probe(root,Path('/unused'),Path('/unused'))
 def test_07_actual_root_inputs_and_only_payload_are_required(self):
    expected=p.native_inputs();self.assertEqual(len(expected),3)
    self.assertIn('crates/horizon-client/src/mod_content.rs',expected);self.assertIn('Cargo.lock',expected)
    self.assertIn('crates/horizon-mixins-core/src/scopes.rs',p.IR_SOURCE_PINS)
    c=m.inputs();self.assertEqual(c['probe_payload_sha256'],{k:m.sha(b) for k,b in payload().items()})
 def test_08_synthetic_acceptance_does_not_claim_native_type(self):
    result=verify(synthetic());self.assertTrue(result['passed']);self.assertFalse(result['native_type_verified']);self.assertFalse(result['full_composition_tested'])
    self.assertEqual(result['selected_plan']['transformer'],'synthetic-only')
 def test_09_extra_work_and_missing_tool_identity_reject(self):
    for key in ['injections','modules','states','crates','bridges','data','post_build']:
        v=synthetic();v[key]=v.get(key,[])+[{}]
        with self.assertRaises(ValueError):verify(v)
    for key in ['rustc','transformer']:
        v=synthetic();v[key]=''
        with self.assertRaises(ValueError):verify(v)
 def test_10_owner_payload_source_node_reject(self):
    for key,value in [('target','other'),('mod_id','other'),('helper_source',''),('node',[-1]),('node',[True])]:
        v=synthetic();v['injections'][0][key]=value
        with self.assertRaises(ValueError):verify(v)
    for path in p.native_inputs():
        v=synthetic();v['inputs'][path]='0'*64
        with self.assertRaises(ValueError):verify(v)
 def test_11_extra_or_missing_capture_rejects(self):
    for key,value in [('captures',[]),('captures',[{},{}]),('call_site',1),('hook','other')]:
        v=synthetic();v['injections'][0][key]=value
        with self.assertRaises(ValueError):verify(v)
 def test_12_owned_out_store_contract(self):
    for key,value in [('ordinal',True),('require',2),('variable_site',4),('cancellable',True)]:
        v=synthetic();v['injections'][0][key]=value
        with self.assertRaises(ValueError):verify(v)
    v=synthetic();v['injections'][0]['contract']['value']='u8'
    with self.assertRaises(ValueError):verify(v)
 def test_13_unsafe_file_input_map_and_local_fingerprint(self):
    with tempfile.TemporaryDirectory() as tmp:
        f=Path(tmp)/'x';f.write_text('{}');link=Path(tmp)/'l';link.symlink_to(f)
        with self.assertRaises(ValueError):p.verify_plan(link,Path('/unused'),Path('/unused'))
    v=synthetic();v['inputs']['../x']='1'*64
    with self.assertRaises(ValueError):verify(v)
    v=synthetic();v['injections'][0]['captures'][0]['local']['fingerprint']='bad'
    with self.assertRaises(ValueError):verify(v)
 def test_14_first_rejection_is_diagnostic_once(self):
    with tempfile.TemporaryDirectory() as tmp:
        work=Path(tmp);(work/'official-plan.log').write_text(json.dumps({'code':'E_BINDING','message':'actual first rejection'}))
        r=mock.Mock(work=work,report={'phases':[{'name':'official-plan','exit_code':1,'error':'RuntimeError: official-plan exited 1'}]});r.run.side_effect=RuntimeError('official-plan exited 1')
        result=m.attempt_plan(r,Path('/c'),Path('/h'),Path('/m'),work/'plan.json')
        self.assertFalse(result['accepted']);self.assertEqual(result['diagnostic']['code'],'E_BINDING');self.assertEqual(r.run.call_count,1)
 def test_15_timeout_not_retried(self):
    r=mock.Mock(work=Path('/work'));r.run.side_effect=TimeoutError('official-plan')
    with self.assertRaises(TimeoutError):m.attempt_plan(r,Path('/c'),Path('/h'),Path('/m'),Path('/p'))
    self.assertEqual(r.run.call_count,1);self.assertEqual(r.run.call_args.kwargs['timeout'],600)
 def test_16_metadata_only_after_cli_compile(self):
    manifest=Path('/official/Cargo.toml');self.assertTrue(m.prepare.allowed('cargo',m.prepare.metadata_args(manifest),manifest))
    for cmd in [['build'],['check'],['test']]:self.assertFalse(m.prepare.allowed('cargo',cmd,manifest))
    self.assertFalse(m.prepare.allowed('rustc',['--crate-name','game'],manifest))
    src=(HERE/'run.py').read_text();self.assertLess(src.index("runner.run('cli-cold-build'"),src.index('prepare.write_wrappers'));self.assertLess(src.index('prepare.write_wrappers'),src.index('result=attempt_plan'))
 def test_17_no_apply_or_complete_composition_and_exact_cli(self):
    tree=ast.parse((HERE/'run.py').read_text());execute=next(n for n in tree.body if isinstance(n,ast.FunctionDef) and n.name=='execute')
    calls=[n for n in ast.walk(execute) if isinstance(n,ast.Call)]
    self.assertEqual(sum(isinstance(n.func,ast.Name) and n.func.id=='attempt_plan' for n in calls),1);self.assertIs(m.Runner,m.baseline.Runner)
    phases=[n.args[0].value for n in calls if isinstance(n.func,ast.Attribute) and n.func.attr=='run' and n.args and isinstance(n.args[0],ast.Constant)]
    for name in ['apply','complete-guards','main-package','common-package']:self.assertNotIn(name,phases)
    good='Running `rustc --crate-name horizon_mixins crates/horizon-mixins-cli/src/main.rs`'
    self.assertEqual(m.cli_witness(good)['crate'],'horizon_mixins')
    with self.assertRaises(ValueError):m.cli_witness(good+'\nRunning `rustc --crate-name horizon_client src/main.rs`')
 def test_18_missing_gates_cannot_pass(self):
    def fake(r):r.update(diagnostic_complete=True,source_verified=True,plan_accepted=True)
    with tempfile.TemporaryDirectory() as tmp,mock.patch.object(m,'execute',side_effect=fake),mock.patch.dict(os.environ,{'GITHUB_STEP_SUMMARY':str(Path(tmp)/'summary')}),contextlib.redirect_stdout(io.StringIO()):
        with self.assertRaises(SystemExit):m.main()
 def test_19_resolved_stamp_capture_exact(self):
    for key,value in [('binding','out'),('expression','&mut stamp'),('expression','&roots'),('ty','&str')]:
        v=synthetic();v['injections'][0]['captures'][0][key]=value
        with self.assertRaises(ValueError):verify(v)
    for key,value in [('id',True),('id',8),('name','roots'),('ordinal',1),('parameter',True),('mutable',True),('supported',False),('ty','u8')]:
        v=synthetic();v['injections'][0]['captures'][0]['local'][key]=value
        with self.assertRaises(ValueError):verify(v)
 def test_20_no_plain_binding_substitution(self):
    for key,value in [('value','&stamp'),('value','$local_ref:stamp:1'),('value','$local_mut:stamp:0'),('ty','&str')]:
        v=synthetic();v['injections'][0]['bindings'][1][key]=value
        with self.assertRaises(ValueError):verify(v)
 def test_21_native_output_decl_and_shape_not_numeric_only(self):
    for key,value in [('id',True),('name','stamp'),('ordinal',True),('parameter',True),('mutable',True),('supported',False),('ty','PathBuf')]:
        v=synthetic();v['injections'][0]['variable']['local'][key]=value
        with self.assertRaises(ValueError):verify(v)
    v=synthetic();v['injections'][0]['captures'][0]['extra']='not schema6'
    with self.assertRaises(ValueError):verify(v)

if __name__=='__main__':unittest.main()
