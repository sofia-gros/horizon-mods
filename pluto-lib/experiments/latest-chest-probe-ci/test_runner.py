import ast,copy,importlib.util,json,os,tempfile,tomllib,unittest
from pathlib import Path
from unittest import mock
HERE=Path(__file__).resolve().parent
s=importlib.util.spec_from_file_location('chest_probe_run',HERE/'run.py');m=importlib.util.module_from_spec(s);s.loader.exec_module(m)
p=m.probe
def plan_value():
    out=p.payload();a=tomllib.loads(out[p.candidate.MANIFEST].decode());d=a['injection'][0]
    return {'schema':6,'transformer':'synthetic-only','rustc':'synthetic-only','inputs':p.required_plan_inputs(),
      'injections':[{'mod_id':'tconstruct_native','mod_version':a['mod']['version'],'injection_id':d['id'],'target':d['target'],
       'kind':'MODIFY_VARIABLE','file':'crates/horizon-client/src/game/block_items.rs','cancellable':False,'priority':a['mod']['priority'],
       'contract':dict(p.CONTRACT),'helper_source':out[p.candidate.HELPER].decode(),'bindings':d['bindings'],
       'ordinal':3,'require':4,'site_fingerprint':p.sha(b'stack'),'variable_site':123,'expression_site':123,'node':[9,9],
       'function_fingerprint':'1'*64,'variable':{'at':'LOAD','local':{'id':7,'name':'stack','ordinal':0,'parameter':False,'mutable':True,'supported':True,'ty':None,'fingerprint':'2'*64}}}]}
def verify(value):
    with tempfile.TemporaryDirectory() as d:
        f=Path(d)/'plan.json';f.write_text(json.dumps(value));return p.verify_plan(f)
class Tests(unittest.TestCase):
 def test_01_source_pins(self):m.validate(m.inputs())
 def test_02_wrong_source_or_branch(self):
    for key in ['repository','branch','candidate_source']:
        v=copy.deepcopy(m.inputs());v[key]='wrong'
        with self.assertRaises(ValueError):m.validate(v)
 def test_03_exact_latest_official(self):
    self.assertEqual(m.OFFICIAL['horizon']['revision'],'41f822b659789780f58d32c63ddfb6887ea1b8bf')
    v=copy.deepcopy(m.inputs());v['official']['horizon']['revision']=m.baseline.OFFICIAL['horizon']['revision']
    with self.assertRaises(ValueError):m.validate(v)
 def test_04_exact_event(self):
    env={'GITHUB_ACTIONS':'true','GITHUB_REPOSITORY':m.REPOSITORY,'GITHUB_EVENT_NAME':'push','GITHUB_REF':'refs/heads/'+m.BRANCH};event={'repository':{'private':False}}
    self.assertTrue(m.event_allowed(env,event))
    for key in env:
        wrong=dict(env);wrong[key]='wrong';self.assertFalse(m.event_allowed(wrong,event))
    self.assertFalse(m.event_allowed(env,{'repository':{'private':True}}))
 def test_05_payload_two_files_only(self):
    value=p.payload();self.assertEqual(set(value),{p.candidate.MANIFEST,p.candidate.HELPER})
    main=tomllib.loads(value[p.candidate.MANIFEST].decode());self.assertEqual(set(main),{'mod','compatibility','injection'})
    self.assertEqual(main['compatibility']['horizon_revision'],p.HORIZON)
 def test_06_revision_inverse_and_helper_exact(self):
    value,r=p.unsealed_payload();self.assertTrue(r['inverse_verified']);self.assertEqual(value[p.candidate.HELPER],(p.candidate.HERE/'source-chest-pick.rs').read_bytes())
    self.assertNotEqual(r['original_header_sha256'],r['candidate_header_sha256'])
 def test_07_write_once(self):
    with tempfile.TemporaryDirectory() as d:
        path=Path(d)/'mods';result=p.write_probe(path);self.assertEqual(result['members'],2)
        self.assertEqual({f.relative_to(path).as_posix():f.read_bytes() for f in path.rglob('*') if f.is_file()},p.payload())
        with self.assertRaises(ValueError):p.write_probe(path)
 def test_08_selected_ir_not_type_proof(self):
    r=verify(plan_value());self.assertTrue(r['passed']);self.assertFalse(r['native_type_verified']);self.assertFalse(r['full_composition_tested']);self.assertEqual(r['input_count'],2)
 def test_09_extra_work_rejected(self):
    for key in ['injections','modules','states','bridges','crates','data','post_build']:
        v=plan_value();v[key]=v.get(key,[])+[{}]
        with self.assertRaises(ValueError):verify(v)
 def test_10_require_ordinal_or_kind_drift(self):
    for k,b in [('require',3),('ordinal',0),('kind','RETURN'),('site_fingerprint','1'*64),('priority',0)]:
        v=plan_value();v['injections'][0][k]=b
        with self.assertRaises(ValueError):verify(v)
 def test_11_binding_and_contract_drift(self):
    for k,b in [('bindings',[]),('contract',{'output':'Option < ItemStack >','value':'Option < ItemStack >','operation':None}),('helper_source','fn nop() {}'),('target','other'),('cancellable',True)]:
        v=plan_value();v['injections'][0][k]=b
        with self.assertRaises(ValueError):verify(v)
 def test_12_local_identity(self):
    for k,b in [('name','other'),('ordinal',1),('parameter',True),('supported',False),('mutable',False),('ty','Other'),('id',-1)]:
        v=plan_value();v['injections'][0]['variable']['local'][k]=b
        with self.assertRaises(ValueError):verify(v)
 def test_13_wrong_site_or_capture(self):
    for k,b in [('captures',[{}]),('variable_site',-1),('call_site',0),('expression_site',0),('node',[])]:
        v=plan_value();v['injections'][0][k]=b
        with self.assertRaises(ValueError):verify(v)
    v=plan_value();v['injections'][0]['variable']['at']='STORE'
    with self.assertRaises(ValueError):verify(v)
 def test_14_required_inputs(self):
    for key in p.required_plan_inputs():
        v=plan_value();del v['inputs'][key]
        with self.assertRaises(ValueError):verify(v)
        v=plan_value();v['inputs'][key]='0'*64
        with self.assertRaises(ValueError):verify(v)
 def test_15_unsafe_plan(self):
    with tempfile.TemporaryDirectory() as d:
        f=Path(d)/'file';f.write_text('{}');l=Path(d)/'link';l.symlink_to(f)
        with self.assertRaises(ValueError):p.verify_plan(l)
    for k,b in [('inputs',{}),('inputs',{'../escape':'1'*64}),('schema',5)]:
        v=plan_value();v[k]=b
        with self.assertRaises(ValueError):verify(v)
 def test_16_first_rejection_preserved(self):
    with tempfile.TemporaryDirectory() as d:
        path=Path(d);(path/'official-plan.log').write_text(json.dumps({'code':'E_BINDING','message':'first rejection'}))
        r=mock.Mock(work=path,report={'phases':[{'name':'official-plan','exit_code':1,'error':'RuntimeError: official-plan exited 1'}]});r.run.side_effect=RuntimeError('official-plan exited 1')
        out=m.attempt_plan(r,Path('/cli'),Path('/official'),Path('/mods'),path/'plan.json');self.assertFalse(out['accepted']);self.assertEqual(out['diagnostic']['code'],'E_BINDING');self.assertEqual(r.run.call_count,1)
 def test_17_timeout_no_retry(self):
    r=mock.Mock(work=Path('/w'),report={'phases':[{'name':'official-plan','exit_code':-15,'error':'TimeoutError'}]});r.run.side_effect=TimeoutError('official-plan')
    with self.assertRaises(TimeoutError):m.attempt_plan(r,Path('/c'),Path('/h'),Path('/m'),Path('/p'))
    self.assertEqual(r.run.call_count,1);self.assertEqual(r.run.call_args.kwargs['timeout'],600)
 def test_18_metadata_only_tools(self):
    path=Path('/official/Cargo.toml');self.assertTrue(m.prepare.allowed('cargo',m.prepare.metadata_args(path),path))
    for argv in [['build'],['check'],['test']]:self.assertFalse(m.prepare.allowed('cargo',argv,path))
    self.assertFalse(m.prepare.allowed('rustc',['--crate-name','game'],path))
 def test_19_inherited_guard_unchanged(self):self.assertIs(m.Runner,m.baseline.Runner)
 def test_20_observer_only_identity_diff(self):
    old=(HERE.parent/'latest-mining-probe-ci/observe_plan.py').read_text()
    expected=old.replace('latest-mining-probe','latest-chest-probe').replace("work/'mining-probe'","work/'chest-probe'")
    self.assertEqual((HERE/'observe_plan.py').read_text(),expected)
 def test_21_phase_order_and_final_collection(self):
    text=(HERE/'run.py').read_text();body=text[text.index('def execute('):text.index('def main(')]
    self.assertLess(body.index('runner.guard(reserve='),body.index('work.mkdir()'))
    self.assertLess(body.index("'cli-cold-build'"),body.index('prepare.write_wrappers'))
    self.assertLess(body.index('prepare.write_wrappers'),body.index('result=attempt_plan('))
    self.assertLess(body.index("report['gates']['plan_diagnostic']"),body.index('probe.verify_plan(plan)'))
    final=body[body.index('finally:'):];self.assertIn('collect_tool_calls(',final);self.assertIn('observer.collect(',final);self.assertIn('assert_full_seal',final)
 def test_22_terminal_checks_and_no_apply(self):
    text=(HERE/'run.py').read_text();self.assertIn("set(report['gates'])!=REQUIRED",text);self.assertIn("report['compatibility']=None",text)
    tree=ast.parse(text)
    for n in ast.walk(tree):
        if isinstance(n,ast.Constant) and isinstance(n.value,str):self.assertNotEqual(n.value,'apply')
 def test_23_ir_pins_drift(self):
    with tempfile.TemporaryDirectory() as d:
        root=Path(d)
        for name in p.IR_SOURCE_PINS:
            path=root/name;path.parent.mkdir(parents=True,exist_ok=True);path.write_text('changed')
        with self.assertRaises(ValueError):p.verify_ir_sources(root)
 def test_24_workflow_minimum_permissions(self):
    w=(m.ROOT/'.github/workflows/latest-chest-probe-wip.yml').read_text()
    for text in ['contents: read','persist-credentials: false','runs-on: ubuntu-24.04',m.BRANCH]:self.assertIn(text,w)
    for text in ['upload-artifact','actions/cache','write-all','workflow_dispatch']:self.assertNotIn(text,w)
if __name__=='__main__':unittest.main()
