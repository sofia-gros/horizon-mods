"""Pure synthetic controller/IR regressions. No planner, Cargo, Rust or official writes."""
import ast,contextlib,copy,importlib.util,io,json,os,tempfile,tomllib,unittest
from pathlib import Path
from unittest import mock
HERE=Path(__file__).resolve().parent
spec=importlib.util.spec_from_file_location('language_ci',HERE/'run.py');m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m)
p=m.probe

def payload():
    files={n:(p.placement.HERE/n).read_bytes() for n in ['horizon-mixin.toml','write-head.rs','stamp-store.rs']}
    files['merge.rs']=(p.placement.REPAIR/'merge.rs').read_bytes();return files

def synthetic():
    files=payload();manifest=tomllib.loads(files['horizon-mixin.toml'].decode());rows=[]
    for item in manifest['injection']:
        row={'mod_id':p.placement.MOD,'mod_version':'0.0.0','injection_id':item['id'],'target':item['target'],'kind':item['kind'],
         'bindings':item['bindings'],'priority':100,'file':p.placement.TARGET,'helper_source':files[item['helper']].decode(),
         'cancellable':item.get('cancellable',False),'node':[1,2],'function_fingerprint':'f'*64}
        if item['kind']=='HEAD':row['contract']={'output':'std::ops::ControlFlow<Result<()>>','value':None}
        else:row.update(ordinal=0,require=1,variable={'at':'STORE','local':{'name':'parts','ordinal':0,'parameter':False,'mutable':True,'supported':True}},
         variable_site=3,expression_site=3,site_fingerprint='e'*64,contract={'output':'Vec<String>','value':'Vec<String>'})
        rows.append(row)
    return {'schema':6,'transformer':'synthetic-only','rustc':'synthetic-only','inputs':p.native_inputs(),'injections':rows,
      'modules':[{'mod_id':p.placement.MOD,'mod_version':'0.0.0','module_id':'bounded-merge','file':p.placement.TARGET,'name':'pluto_language_overlay','source':files['merge.rs'].decode(),'fingerprint':'d'*64}]}

def verify(value):
    with tempfile.TemporaryDirectory() as tmp,mock.patch.object(p.placement,'payload',return_value=payload()):
        path=Path(tmp)/'plan.json';path.write_text(json.dumps(value));return p.verify_plan(path,Path('/never-opened-official'))

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
    before=(HERE.parent/'latest-mining-probe-ci/run.py').read_text();d=json.loads((HERE/'derivation.json').read_bytes())
    self.assertEqual(m.sha(before.encode()),d['source_sha256'])
    for item in d['changes']:
        self.assertEqual(before.count(item['before']),item['count']);before=before.replace(item['before'],item['after'])
    self.assertEqual(before,(HERE/'run.py').read_text())
 def test_05_observer_changes_only_branch_and_owned_directory(self):
    before=(HERE.parent/'latest-mining-probe-ci/observe_plan.py').read_text()
    after=(HERE/'observe_plan.py').read_text()
    self.assertEqual(after.replace(m.BRANCH,'checkpoint/plutolib-latest-mining-probe-ci-20261106').replace("work/'language-placement-probe'","work/'mining-probe'"),before)
    self.assertEqual((m.observer.MAX_BYTES,m.observer.MAX_SAMPLES,m.observer.INTERVAL),(65536,128,5.0))
 def test_06_fresh_exact_four_file_output(self):
    files=payload()
    with tempfile.TemporaryDirectory() as tmp,mock.patch.object(p.placement,'payload',return_value=files):
        root=Path(tmp)/'probe';result=p.write_probe(root,Path('/unused'))
        self.assertEqual(result['members'],4);self.assertEqual(result['injections'],2)
        self.assertEqual({x.relative_to(root/p.placement.MOD).as_posix():x.read_bytes() for x in (root/p.placement.MOD).iterdir()},files)
        with self.assertRaises(ValueError):p.write_probe(root,Path('/unused'))
 def test_07_actual_language_inputs_and_policy_payload_are_required(self):
    expected=p.native_inputs();self.assertEqual(len(expected),7)
    self.assertIn('crates/horizon-mods/src/content.rs',expected);self.assertIn('Cargo.lock',expected)
    c=m.inputs();self.assertEqual(c['probe_payload_sha256'],{p.placement.MOD+'/'+k:m.sha(b) for k,b in payload().items()})
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
 def test_11_no_capture_or_other_callsite(self):
    for key,value in [('captures',[{}]),('call_site',1),('hook','other')]:
        v=synthetic();v['injections'][0][key]=value
        with self.assertRaises(ValueError):verify(v)
 def test_12_cancellable_head_and_owned_store_contract(self):
    v=synthetic();v['injections'][0]['cancellable']=False
    with self.assertRaises(ValueError):verify(v)
    for key,value in [('ordinal',True),('require',2),('variable_site',4)]:
        v=synthetic();v['injections'][1][key]=value
        with self.assertRaises(ValueError):verify(v)
    v=synthetic();v['injections'][1]['contract']['value']='u8'
    with self.assertRaises(ValueError):verify(v)
 def test_13_unsafe_file_input_map_and_module_fingerprint(self):
    with tempfile.TemporaryDirectory() as tmp:
        f=Path(tmp)/'x';f.write_text('{}');link=Path(tmp)/'l';link.symlink_to(f)
        with self.assertRaises(ValueError):p.verify_plan(link,Path('/unused'))
    v=synthetic();v['inputs']['../x']='1'*64
    with self.assertRaises(ValueError):verify(v)
    v=synthetic();v['modules'][0]['fingerprint']='bad'
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

if __name__=='__main__':unittest.main()
