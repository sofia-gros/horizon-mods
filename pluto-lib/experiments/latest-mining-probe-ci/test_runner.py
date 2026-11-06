import ast,contextlib,copy,importlib.util,io,json,os,tempfile,tomllib,unittest
from pathlib import Path
from unittest import mock
HERE=Path(__file__).resolve().parent
s=importlib.util.spec_from_file_location('mining_probe_ci',HERE/'run.py');m=importlib.util.module_from_spec(s);s.loader.exec_module(m)
p=m.probe

def plan_value():
    payload=p.payload();a=tomllib.loads(payload[p.candidate.MANIFEST].decode());b=tomllib.loads(payload[p.candidate.COMMON_MANIFEST].decode());d=a['injection'][0]
    return {'schema':6,'transformer':'fixture-only','rustc':'fixture-only','inputs':p.required_plan_inputs(),
      'injections':[{'mod_id':'tconstruct_native','mod_version':a['mod']['version'],'injection_id':d['id'],'target':d['target'],'kind':'HEAD','file':'crates/horizon-client/src/game/native_hooks.rs','cancellable':False,'priority':a['mod']['priority'],'contract':dict(p.VOID_HEAD_CONTRACT),'helper_source':payload[p.candidate.HELPER].decode(),'bindings':d['bindings'],'node':[1,2],'function_fingerprint':'2'*64}],
      'modules':[{'mod_id':'pluto_native_creative','mod_version':b['mod']['version'],'module_id':'mining-context-facts','file':'crates/horizon-client/src/game.rs','name':'pluto_mining_context','source':payload[p.candidate.COMMON_SOURCE].decode(),'fingerprint':'3'*64}]}

def verify(value):
    with tempfile.TemporaryDirectory() as d:
        f=Path(d)/'plan.json';f.write_text(json.dumps(value));return p.verify_plan(f)

class Tests(unittest.TestCase):
 def test_01_exact_source_chain(self):m.validate(m.inputs())
 def test_02_other_sources_and_pins_rejected(self):
    for key,value in [('branch','other'),('candidate_source','0'*40),('repository','other/repo')]:
        c=m.inputs();c[key]=value
        with self.assertRaises(ValueError):m.validate(c)
    c=m.inputs();c['official']['horizon']['revision']='0'*40
    with self.assertRaises(ValueError):m.validate(c)
 def test_03_source_file_set_rejected(self):
    c=m.inputs();c['source_sha256'].pop(next(iter(c['source_sha256'])))
    with self.assertRaises(ValueError):m.validate(c)
 def test_04_only_public_exact_trigger(self):
    e={'GITHUB_ACTIONS':'true','GITHUB_REPOSITORY':m.REPOSITORY,'GITHUB_EVENT_NAME':'push','GITHUB_REF':'refs/heads/'+m.BRANCH}
    self.assertTrue(m.event_allowed(e,{'repository':{'private':False}}))
    self.assertFalse(m.event_allowed({},{}));self.assertFalse(m.event_allowed(e,{'repository':{'private':True}}))
 def test_05_exact_two_native_source_files_no_policy_stub(self):
    source=p.payload();self.assertEqual(len(source),4)
    self.assertEqual(source[p.candidate.HELPER],(p.candidate.HERE/'source-mining-context-native.rs').read_bytes())
    self.assertEqual(source[p.candidate.COMMON_SOURCE],(p.candidate.HERE/'mining-context.rs').read_bytes())
    self.assertFalse(any(name.endswith('source-mining-context-shared.rs') for name in source))
 def test_06_one_head_one_internal_module_no_other_work(self):
    source=p.payload();a=tomllib.loads(source[p.candidate.MANIFEST].decode());b=tomllib.loads(source[p.candidate.COMMON_MANIFEST].decode())
    self.assertEqual(len(a['injection']),1);self.assertEqual(len(b['module']),1)
    self.assertEqual(b['module'][0]['visibility'],'pub(crate)')
    for key in ['state','crate','data','post_build']:self.assertNotIn(key,a);self.assertNotIn(key,b)
 def test_07_only_fresh_owned_probe_output(self):
    with tempfile.TemporaryDirectory() as d:
        root=Path(d)/'mods';result=p.write_probe(root);self.assertEqual(result['members'],4)
        self.assertEqual({f.relative_to(root).as_posix():f.read_bytes() for f in root.rglob('*') if f.is_file()},p.payload())
        with self.assertRaises(ValueError):p.write_probe(root)
 def test_08_selected_plan_validation_is_not_type_proof(self):
    result=verify(plan_value());self.assertTrue(result['passed']);self.assertFalse(result['native_type_verified']);self.assertFalse(result['full_composition_tested'])
    self.assertEqual(result['input_count'],5)
 def test_09_extra_plan_work_rejected(self):
    for key in ['injections','modules','states','bridges','crates','data','post_build']:
        x=plan_value();x[key]=x.get(key,[])+[{}]
        with self.assertRaises(ValueError):verify(x)
 def test_10_wrong_module_owner_visibility_rejected(self):
    for key,v in [('visibility','pub'),('mod_id','tconstruct_native'),('name','other'),('file','other.rs')]:
        x=plan_value();x['modules'][0][key]=v
        with self.assertRaises(ValueError):verify(x)
 def test_11_wrong_helper_or_target_rejected(self):
    for key,v in [('helper_source','fn no_op() {}'),('target','other'),('kind','RETURN'),('cancellable',True),('file','other.rs')]:
        x=plan_value();x['injections'][0][key]=v
        with self.assertRaises(ValueError):verify(x)
 def test_12_lexical_or_original_bindings_rejected(self):
    x=plan_value();x['injections'][0]['captures']=[{}]
    with self.assertRaises(ValueError):verify(x)
    x=plan_value();x['injections'][0]['bindings'][-1]['value']='original'
    with self.assertRaises(ValueError):verify(x)
    x=plan_value();x['injections'][0]['variable_site']=0
    with self.assertRaises(ValueError):verify(x)
 def test_13_plan_file_and_fingerprint_bounds(self):
    with tempfile.TemporaryDirectory() as d:
        f=Path(d)/'file';f.write_text('{}');link=Path(d)/'link';link.symlink_to(f)
        with self.assertRaises(ValueError):p.verify_plan(link)
    for field,v in [('inputs',{}),('inputs',{'../escape':'1'*64}),('schema',5)]:
        x=plan_value();x[field]=v
        with self.assertRaises(ValueError):verify(x)
 def test_14_first_rejection_is_preserved_once(self):
    with tempfile.TemporaryDirectory() as d:
        work=Path(d);(work/'official-plan.log').write_text(json.dumps({'code':'E_BINDING','message':'actual first rejection'}))
        r=mock.Mock(work=work,report={'phases':[{'name':'official-plan','exit_code':1,'error':'RuntimeError: official-plan exited 1'}]});r.run.side_effect=RuntimeError('official-plan exited 1')
        out=m.attempt_plan(r,Path('/cli'),Path('/official'),Path('/mods'),work/'plan.json')
        self.assertFalse(out['accepted']);self.assertEqual(out['diagnostic']['code'],'E_BINDING');self.assertEqual(r.run.call_count,1)
 def test_15_timeout_has_no_diagnostic_or_retry(self):
    r=mock.Mock(work=Path('/work'),report={'phases':[{'name':'official-plan','exit_code':-15,'error':'TimeoutError: official-plan'}]});r.run.side_effect=TimeoutError('official-plan')
    with self.assertRaises(TimeoutError):m.attempt_plan(r,Path('/c'),Path('/h'),Path('/m'),Path('/p'))
    self.assertEqual(r.run.call_count,1);self.assertEqual(r.run.call_args.kwargs['timeout'],600)
 def test_16_metadata_only_after_tool_build(self):
    manifest=Path('/official/Cargo.toml');self.assertTrue(m.prepare.allowed('cargo',m.prepare.metadata_args(manifest),manifest))
    for cmd in [['build'],['check'],['test']]:self.assertFalse(m.prepare.allowed('cargo',cmd,manifest))
    self.assertFalse(m.prepare.allowed('rustc',['--crate-name','game'],manifest))
 def test_17_observer_diff_is_only_branch_and_owned_directory(self):
    before=(HERE.parent/'latest-main-common-ci/observe_plan.py').read_text();after=(HERE/'observe_plan.py').read_text()
    self.assertEqual(after.replace(m.BRANCH,m.baseline.BRANCH).replace("work/'mining-probe'","work/'revision-only-candidate'"),before)
    self.assertEqual(m.observer.MAX_BYTES,65536);self.assertEqual(m.observer.MAX_SAMPLES,128);self.assertEqual(m.observer.INTERVAL,5.0)
 def test_18_actual_cli_witness_and_no_game_compile(self):
    cli='Running `rustc --crate-name horizon_mixins crates/horizon-mixins-cli/src/main.rs`'
    self.assertEqual(m.cli_witness(cli)['crate'],'horizon_mixins')
    for text in ['',cli+'\n'+cli,cli+'\nRunning `rustc --crate-name horizon_client src/main.rs`']:
        with self.assertRaises(ValueError):m.cli_witness(text)
 def test_19_no_apply_or_complete_plan_guards(self):
    tree=ast.parse((HERE/'run.py').read_text());execute=next(n for n in tree.body if isinstance(n,ast.FunctionDef) and n.name=='execute')
    calls=[n for n in ast.walk(execute) if isinstance(n,ast.Call)]
    phases=[n.args[0].value for n in calls if isinstance(n.func,ast.Attribute) and n.func.attr=='run' and n.args and isinstance(n.args[0],ast.Constant)]
    self.assertNotIn('complete-guards',phases);self.assertNotIn('main-package',phases);self.assertNotIn('common-package',phases)
    self.assertEqual(sum(isinstance(n.func,ast.Name) and n.func.id=='attempt_plan' for n in calls),1)
    self.assertIs(m.Runner,m.baseline.Runner)
 def test_20_terminal_cannot_pass_with_missing_gates(self):
    def false_success(report):report.update(diagnostic_complete=True,source_verified=True,plan_accepted=True)
    with tempfile.TemporaryDirectory() as d,mock.patch.object(m,'execute',side_effect=false_success),mock.patch.dict(os.environ,{'GITHUB_STEP_SUMMARY':str(Path(d)/'summary')}),contextlib.redirect_stdout(io.StringIO()):
        with self.assertRaises(SystemExit) as exc:m.main()
        self.assertEqual(exc.exception.code,1)
 def test_21_required_input_map_retained_exactly(self):
    result=verify(plan_value());self.assertEqual(result['required_inputs'],p.required_plan_inputs());self.assertEqual(len(result['required_inputs']),5)
    self.assertEqual(result['required_inputs']['Cargo.lock'],m.OFFICIAL['horizon']['lock'])
 def test_22_each_missing_required_input_rejected(self):
    for key in p.required_plan_inputs():
        x=plan_value();del x['inputs'][key]
        with self.subTest(path=key),self.assertRaises(ValueError):verify(x)
 def test_23_each_changed_required_input_rejected(self):
    for key in p.required_plan_inputs():
        x=plan_value();x['inputs'][key]='0'*64
        with self.subTest(path=key),self.assertRaises(ValueError):verify(x)
 def test_24_actual_owner_priority_required(self):
    source=p.payload();a=tomllib.loads(source[p.candidate.MANIFEST].decode())
    self.assertEqual(a['mod']['priority'],100);self.assertNotIn('priority',a['injection'][0])
    for wrong in [None,0,99]:
        x=plan_value()
        if wrong is None:del x['injections'][0]['priority']
        else:x['injections'][0]['priority']=wrong
        with self.assertRaises(ValueError):verify(x)
 def test_25_non_cancellable_head_contract_exact(self):
    for wrong in [None,{}, {'output':'::core::ops::ControlFlow < () >','value':None,'operation':None},
                  {'output':'()','value':'f32','operation':None}, {**p.VOID_HEAD_CONTRACT,'bundle':None}]:
        x=plan_value();x['injections'][0]['contract']=wrong
        with self.assertRaises(ValueError):verify(x)
 def test_26_ir_source_missing_and_byte_drift_rejected(self):
    with tempfile.TemporaryDirectory() as d:
        with self.assertRaises(FileNotFoundError):p.verify_ir_sources(Path(d))
        for name in p.IR_SOURCE_PINS:
            f=Path(d)/name;f.parent.mkdir(parents=True,exist_ok=True);f.write_bytes(b'changed')
        with self.assertRaises(ValueError):p.verify_ir_sources(Path(d))
 def test_27_ir_contract_pin_drift_rejected_before_files(self):
    config=m.inputs();config['ir_contract_source_sha256']={}
    with mock.patch.object(p.json,'loads',return_value=config),self.assertRaisesRegex(ValueError,'IR contract source pins'):
        p.verify_ir_sources(Path('/not-read'))
if __name__=='__main__':unittest.main()
