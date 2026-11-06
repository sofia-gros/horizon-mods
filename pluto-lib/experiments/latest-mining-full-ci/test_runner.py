import ast,contextlib,copy,importlib.util,io,json,os,tempfile,tomllib,unittest
from pathlib import Path
from unittest import mock
HERE=Path(__file__).resolve().parent
s=importlib.util.spec_from_file_location('full_controller',HERE/'run.py');m=importlib.util.module_from_spec(s);s.loader.exec_module(m)
f=m.full;c=f.candidate
s=importlib.util.spec_from_file_location('small_unit_helpers',HERE.parent/'latest-mining-probe-ci/test_runner.py');small_tests=importlib.util.module_from_spec(s);s.loader.exec_module(small_tests)

def maps():
    revised={c.MANIFEST:c.revision_only_manifest(),c.COMMON_MANIFEST:c.common_revision_only_manifest(),'unchanged.bin':b'opaque'}
    after={**revised,c.MANIFEST:c.transform_manifest(revised[c.MANIFEST]),c.COMMON_MANIFEST:c.transform_common_manifest(revised[c.COMMON_MANIFEST]),c.HELPER:b'exact helper checked elsewhere',c.COMMON_SOURCE:b'exact module checked elsewhere'}
    original=revised.copy()
    for row in m.prepare.contract()['manifest_changes']:original[row['path']]=original[row['path']].replace(row['after_line'].encode(),row['before_line'].encode(),1)
    return original,revised,after

def verify_plan(value):
    with tempfile.TemporaryDirectory() as d:
        root=Path(d);mods=root/'mods';mods.mkdir()
        for name,body in f.small.payload().items():
            p=mods/name;p.parent.mkdir(parents=True,exist_ok=True);p.write_bytes(body)
        p=root/'plan.json';p.write_text(json.dumps(value));return f.verify_plan(p,mods)

def plan_value():
    v=small_tests.plan_value();v.update(states=[],crates=[]);return v

class Tests(unittest.TestCase):
 def test_01_exact_source_chain(self):m.validate(m.inputs())
 def test_02_wrong_source_and_branch(self):
    for k in ['candidate_source','collector_source','branch','repository']:
        v=m.inputs();v[k]='wrong'
        with self.assertRaises(ValueError):m.validate(v)
 def test_03_source_file_set_and_pin_drift(self):
    v=m.inputs();v['source_sha256'].pop(next(iter(v['source_sha256'])))
    with self.assertRaises(ValueError):m.validate(v)
    v=m.inputs();v['official']['horizon']['revision']='0'*40
    with self.assertRaises(ValueError):m.validate(v)
 def test_04_public_exact_event_only(self):
    e={'GITHUB_ACTIONS':'true','GITHUB_REPOSITORY':m.REPOSITORY,'GITHUB_EVENT_NAME':'push','GITHUB_REF':'refs/heads/'+m.BRANCH}
    self.assertTrue(m.event_allowed(e,{'repository':{'private':False}}));self.assertFalse(m.event_allowed(e,{'repository':{'private':True}}));self.assertFalse(m.event_allowed({},{}))
 def test_05_unchanged_runner_guards_and_wrappers(self):
    self.assertIs(m.Runner,m.baseline.Runner);self.assertIs(m.attempt_guards,m.baseline.attempt_guards);self.assertIs(m.collect_tool_calls,m.baseline.collect_tool_calls)
    for argv in [['build'],['check'],['test']]:self.assertFalse(m.prepare.allowed('cargo',argv,Path('/official/Cargo.toml')))
 def test_06_observer_only_branch_and_owned_directory_delta(self):
    old=(HERE.parent/'latest-main-common-ci/observe_plan.py').read_text();new=(HERE/'observe_plan.py').read_text()
    self.assertEqual(new.replace(m.BRANCH,m.baseline.BRANCH).replace("work/'mining-candidate'","work/'revision-only-candidate'"),old)
    self.assertEqual(m.observer.MAX_BYTES,65536);self.assertEqual(m.observer.MAX_SAMPLES,128)
 def test_07_tree_links_and_special_paths_rejected(self):
    with tempfile.TemporaryDirectory() as d:
        p=Path(d);(p/'link').symlink_to('/tmp')
        with self.assertRaises(ValueError):f.read_tree(p)
 def test_08_exact_small_inverse_composition(self):
    original,_,after=maps();self.assertEqual(f.restore_original(after,m.prepare.contract()['manifest_changes']),original)
 def test_09_inverse_ambiguity_or_unrelated_change_detected(self):
    original,_,after=maps();after[c.MANIFEST]+=c.NEW_BLOCK
    with self.assertRaises(ValueError):f.restore_original(after,m.prepare.contract()['manifest_changes'])
    original,_,after=maps();after['unchanged.bin']=b'changed'
    self.assertNotEqual(f.restore_original(after,m.prepare.contract()['manifest_changes']),original)
 def test_10_incomplete_payload_fails_before_output(self):
    with tempfile.TemporaryDirectory() as d:
        p=Path(d);(p/'original').mkdir();(p/'revised').mkdir()
        with self.assertRaises(ValueError):f.write_candidate(p/'original',p/'revised',p/'out',m.prepare.contract()['manifest_changes'])
        self.assertFalse((p/'out').exists())
 def test_11_duplicate_owner_rejected(self):
    payload=f.small.payload();payload['duplicate/horizon-mixin.toml']=payload[c.MANIFEST]
    with self.assertRaises(ValueError):f.inventory(payload)
 def test_12_synthetic_selected_contract_only(self):
    v=verify_plan(plan_value());self.assertTrue(v['passed']);self.assertFalse(v['native_type_verified']);self.assertEqual(v['counts'],{'injections':1,'modules':1,'states':0,'crates':0})
 def test_13_wrong_complete_counts_rejected(self):
    for key in ['injections','modules','states','crates']:
        v=plan_value();v[key].append({})
        with self.assertRaises(ValueError):verify_plan(v)
 def test_14_wrong_complete_owners_rejected(self):
    for key,name in [('injections','injection_id'),('modules','module_id')]:
        v=plan_value();v[key][0][name]='wrong'
        with self.assertRaises(ValueError):verify_plan(v)
 def test_15_mining_binding_priority_contract_rejected(self):
    for key,val in [('priority',0),('contract',{}),('kind','RETURN'),('captures',[{}]),('helper_source','stub')]:
        v=plan_value();v['injections'][0][key]=val
        with self.assertRaises(ValueError):verify_plan(v)
 def test_16_each_official_input_required(self):
    for key in f.small.required_plan_inputs():
        v=plan_value();del v['inputs'][key]
        with self.assertRaises(ValueError):verify_plan(v)
 def test_17_plan_links_and_fingerprint_rejected(self):
    with tempfile.TemporaryDirectory() as d:
        p=Path(d);(p/'f').write_text('{}');(p/'link').symlink_to(p/'f')
        with self.assertRaises(ValueError):f.plan_bytes(p/'link')
    for key in ['function_fingerprint','node']:
        v=plan_value();v['injections'][0][key]=None
        with self.assertRaises(ValueError):verify_plan(v)
 def test_18_one_rejected_plan_no_retry(self):
    with tempfile.TemporaryDirectory() as d:
        p=Path(d);(p/'official-plan.log').write_text(json.dumps({'code':'E_LOCAL_NOT_FOUND','message':'actual'}));r=mock.Mock(work=p,report={'phases':[{'name':'official-plan','exit_code':1,'error':'RuntimeError: official-plan exited 1'}]});r.run.side_effect=RuntimeError('official-plan exited 1')
        out=m.attempt_plan(r,Path('/cli'),Path('/h'),Path('/m'),p/'plan.json');self.assertFalse(out['accepted']);self.assertEqual(r.run.call_count,1)
 def test_19_timeout_remains_exception_no_retry(self):
    r=mock.Mock(work=Path('/tmp'),report={'phases':[]});r.run.side_effect=TimeoutError('official-plan')
    with self.assertRaises(TimeoutError):m.attempt_plan(r,Path('/c'),Path('/h'),Path('/m'),Path('/p'))
    self.assertEqual(r.run.call_count,1);self.assertEqual(r.run.call_args.kwargs['timeout'],600)
 def test_20_collector_receipt_and_explicit_repo_root(self):
    with tempfile.TemporaryDirectory() as d:
        p=Path(d)/'plan';p.write_text('{}');r=mock.Mock();r.run.return_value=json.dumps({'status':'observed_only','compatibility':None,'native_executed':False,'plan':{'sha256':m.sha(b'{}')},'contract_sha256':m.sha((m.ROOT/m.COLLECTOR_PATH/'contract.json').read_bytes())})
        out=m.attempt_terrain(r,p,Path('/official'));self.assertTrue(out['passed']);args=r.run.call_args.args[1];self.assertEqual(args[args.index('--mod-root')+1],str(m.ROOT))
 def test_21_collector_rejection_is_not_compatibility_false(self):
    with tempfile.TemporaryDirectory() as d:
        p=Path(d);plan=p/'plan';plan.write_text('{}');(p/'terrain-witness.log').write_text(json.dumps({'status':'rejected','compatibility':None,'plan_sha256':m.sha(b'{}'),'error':'actual contract refusal'}));r=mock.Mock(work=p,report={'phases':[{'name':'terrain-witness','exit_code':1,'error':'RuntimeError: terrain-witness exited 1'}]});r.run.side_effect=RuntimeError('terrain-witness exited 1')
        out=m.attempt_terrain(r,plan,Path('/h'));self.assertFalse(out['passed']);self.assertIsNone(out['result']['compatibility']);self.assertEqual(r.run.call_count,1)
 def test_22_collector_malformed_result_fails_closed(self):
    with tempfile.TemporaryDirectory() as d:
        p=Path(d)/'p';p.write_text('{}');r=mock.Mock();r.run.return_value='{"status":"observed_only"}'
        with self.assertRaises(ValueError):m.attempt_terrain(r,p,Path('/h'))
 def test_23_evidence_precedes_collector_and_old_guards(self):
    source=(HERE/'run.py').read_text();execute=source[source.index('def execute('):source.index('def main(')]
    positions=[execute.index(s) for s in ["report['gates']['plan_diagnostic']",'artifact_seals[str(plan)]','full.verify_plan(plan,mods)','attempt_terrain(runner,plan,official)','attempt_guards(runner,plan,official)']]
    self.assertEqual(positions,sorted(positions));self.assertIn("if terrain['passed']:",execute)
    tree=ast.parse(execute);calls=[n for n in ast.walk(tree) if isinstance(n,ast.Call)];self.assertEqual(sum(isinstance(n.func,ast.Name) and n.func.id=='attempt_plan' for n in calls),1)
    self.assertNotIn("'apply'",execute);self.assertNotIn("'host-build'",execute)
 def test_24_terminal_requires_conditional_gates(self):
    r={'plan_accepted':True,'gates':{'terrain_witness':{'passed':False}}};self.assertNotIn('complete_guards',m.required_gates(r));r['gates']['terrain_witness']['passed']=True;self.assertIn('complete_guards',m.required_gates(r))
    def false_success(report):report.update(diagnostic_complete=True,source_verified=True,plan_accepted=True)
    with tempfile.TemporaryDirectory() as d,mock.patch.object(m,'execute',side_effect=false_success),mock.patch.dict(os.environ,{'GITHUB_STEP_SUMMARY':str(Path(d)/'summary')}),contextlib.redirect_stdout(io.StringIO()):
        with self.assertRaises(SystemExit) as exc:m.main()
        self.assertEqual(exc.exception.code,1)
if __name__=='__main__':unittest.main()
