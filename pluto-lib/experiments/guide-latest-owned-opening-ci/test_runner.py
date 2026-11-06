import contextlib
import importlib.util
import io
import json
from pathlib import Path
import unittest
from unittest import mock

HERE=Path(__file__).resolve().parent
spec=importlib.util.spec_from_file_location('owned_ci',HERE/'run.py')
m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m)

class Checks(unittest.TestCase):
    def test_pinned_latest_sources_inventory_and_frozen_helper(self):
        value=m.config();old=m.prior.config()
        self.assertEqual(len(value['inventory_names']),17);self.assertEqual(len(value['ownership_names']),10)
        self.assertEqual(value['expected_delta'][m.HELPER],old['expected_delta'][m.HELPER])
        self.assertEqual(value['official']['revision'],m.HORIZON)
        self.assertNotEqual(value['official']['revision'],m.old.HORIZON)
        self.assertEqual(len(value['expected_delta']),13)

    def test_composition_retains_whole_base_and_exact_two_fragments(self):
        original=(m.old.PROTOTYPE/'retained_book.rs').read_bytes()
        expected=original+b'\n'+(m.SOURCE/'owned.rs').read_bytes()+b'\n'+(m.SOURCE/'book-view.rs').read_bytes()
        self.assertEqual(m.composed_helper(),expected)
        self.assertEqual(m.composed_helper().count(b'static NEXT:'),1)

    def test_only_exact_public_trigger(self):
        env={'GITHUB_ACTIONS':'true','GITHUB_REPOSITORY':m.REPOSITORY,'GITHUB_EVENT_NAME':'push','GITHUB_REF':'refs/heads/'+m.BRANCH,'GITHUB_SHA':'head'}
        event={'repository':{'private':False,'full_name':m.REPOSITORY}}
        m.validate_event(env,event,'head')
        for key in env:
            bad=dict(env);bad[key]='different'
            with self.assertRaises(ValueError):m.validate_event(bad,event,'head')
        with self.assertRaises(ValueError):m.validate_event(env,{'repository':{'private':True,'full_name':m.REPOSITORY}},'head')

    def test_inventory_matches_recorded_terse_bytes_and_rejects_any_extra_output(self):
        names=m.config()['inventory_names'];good=''.join(n+': test\n' for n in sorted(names))
        self.assertEqual(len(good.encode()),1589)
        self.assertEqual(m.sha(good.encode()),'6c8956115e348eda2885f926e55c035df43121dd67652cd2ee4100f09c6956c0')
        m.inventory(good,names)
        for bad in ['',good.replace(names[0],names[1]),good+names[0]+': test\n',
                    good+'17 tests, 0 benchmarks\n','warning\n'+good,good+'\n',
                    good.replace(': test',': benchmark',1),good.replace(names[0]+': test\n',''),
                    ''.join(n+': test\n' for n in reversed(sorted(names)))]:
            with self.assertRaises(ValueError):m.inventory(bad,names)
        with self.assertRaises(ValueError):m.inventory(good,names[:-1])
        with self.assertRaises(ValueError):m.inventory(good,names[:-1]+[names[0]])

    def test_exact_ten_seven_filtered_and_no_ignored_or_replayed_rows(self):
        names=m.config()['ownership_names'];good=''.join('test '+n+' ... ok\n' for n in names)+'test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 7 filtered out;\n'
        m.tests(good,names)
        for bad in ['',good+good,good.replace('... ok','... ignored',1),good.replace('7 filtered','0 filtered'),good.replace(names[0],names[1])]:
            with self.assertRaises(ValueError):m.tests(bad,names)

    def test_incomplete_failure_or_summary_error_never_reports_success(self):
        def incomplete(r):r['verified']=True
        def complete(r):r.update(verified=True,gates={k:True for k in m.REQUIRED})
        for execute,output in [(RuntimeError('compile'),mock.mock_open()),(incomplete,mock.mock_open()),(complete,mock.Mock(side_effect=OSError('summary')))]:
            with mock.patch.object(m,'execute',side_effect=execute),mock.patch('builtins.open',output),mock.patch.dict(m.os.environ,{'GITHUB_STEP_SUMMARY':'/summary'}),contextlib.redirect_stdout(io.StringIO()) as stream:
                with self.assertRaises(SystemExit):m.main()
            self.assertFalse(json.loads(stream.getvalue().split('FINAL_RECEIPT ')[1])['passed'])

    def test_prior_checked_process_guard_and_separate_metadata_stdout_are_reused(self):
        source=(HERE/'run.py').read_text()
        self.assertIs(m.Runner,m.old.Runner)
        self.assertIn('runner.guard(reserve=2*1024**3)',source)
        self.assertEqual(source.count('cwd=pristine,separate_stdout=True'),1)
        self.assertEqual(source.count('cwd=experiment,separate_stdout=True'),1)
        self.assertIn("str(STORAGE/'test_runner.py')",source)

    def test_original_prototype_and_pristine_are_not_modified(self):
        source=(HERE/'run.py').read_text()
        self.assertIn('inputs=legacy.read_inputs(pristine,pins)',source)
        self.assertIn('latest.build_known_inputs(inputs,pins)',source)
        self.assertNotIn('old.prepare(pristine,experiment)',source)
        self.assertIn('target=experiment/name',source)
        self.assertNotIn('target = pristine',source)
        self.assertIn('old.verify_delta(pristine,experiment',source)
        self.assertIn('if seal(pristine)!=before',source)

    def test_binary_and_all_source_seals_survive_to_receipt(self):
        source=(HERE/'run.py').read_text()
        for token in ['base.binary_seal(binary)!=compiled','if seal(ROOT)!=source','if seal(Path(name))!=before','old.out_dirs(work/\'target\')!=generated','report[\'verified\']=True']:
            self.assertIn(token,source)
        self.assertIn("'host_type'",m.REQUIRED.__repr__())

    def test_workflow_stays_read_only_dedicated_and_without_extra_services(self):
        source=(m.ROOT/'.github/workflows/guide-latest-owned-opening-wip.yml').read_text()
        for token in ['contents: read','private == false','persist-credentials: false',m.BRANCH,'ubuntu-24.04']:
            self.assertIn(token,source)
        for token in ['workflow_dispatch','pull_request:','actions/cache','upload-artifact','secrets.','self-hosted','contents: write']:
            self.assertNotIn(token,source)
        runner=(HERE/'run.py').read_text()
        for token in ["'sudo'","'apt-get'",'horizon-mixins','package_native.py']:
            self.assertNotIn(token,runner)


    def test_latest_full_inventory_precedes_native_compile_and_copy(self):
        source=(HERE/'run.py').read_text()
        self.assertLess(source.index("runner.run('latest-inventory'"),source.index("runner.run('compile-ownership'"))
        self.assertLess(source.index("runner.run('latest-inventory'"),source.index("runner.run('prepare-experiment'"))
        for gate in ['latest_inventory','prototype_identity','latest_source']:
            self.assertIn(gate,m.REQUIRED)

    def test_partial_or_changed_inventory_receipt_cannot_advance(self):
        partial=json.loads((m.LATEST/'generation.json').read_text())
        with self.assertRaises(ValueError):m.complete_inventory(json.dumps(partial))
        complete=dict(partial,full_inventory_verified=True,status='Full current game direct-site inventory and exact source transform checked; not applied or typed.')
        self.assertEqual(m.complete_inventory(json.dumps(complete)),complete)
        changed=dict(complete,latest_typing_executed=True)
        with self.assertRaises(ValueError):m.complete_inventory(json.dumps(changed))
        with self.assertRaises(ValueError):m.complete_inventory(json.dumps(complete)+'warning')

    def test_export_fetch_and_lock_are_new_target_not_old_pin(self):
        source=(HERE/'run.py').read_text()
        self.assertIn("'archive','--format=tar',HORIZON",source)
        self.assertIn("'fetch','--depth','1','origin',HORIZON",source)
        self.assertIn("sha((pristine/'Cargo.lock').read_bytes())!=LOCK",source)
        self.assertNotIn('old.export(',source)
        self.assertNotIn("origin',old.HORIZON]",source)
        self.assertEqual(m.LOCK,'0bc4234f3001d44df9d46fbb06df331d3f53b1af6b613ec9082baaec3d93c62f')

    def test_reviewed_fixture_transitions_are_pinned_without_changing_old_fragments(self):
        value=m.config();generation=json.loads((m.LATEST/'generation.json').read_text())
        self.assertIn("'count':len(value['expected_delta'])", (HERE/'run.py').read_text())
        self.assertEqual(len(generation['source_sha256']),21)
        self.assertEqual(len(generation['rewrites']),25)
        self.assertEqual(sum(r.get('kind')=='reviewed-fixture-assignment' for r in generation['rewrites']),3)
        for name in ['boat_authority_fixture.rs','boat_rider_fixture.rs']:
            self.assertIn('crates/horizon-client/src/game/'+name,value['expected_delta'])
        self.assertEqual(value['runtime_source_publication'],'0fa3fd013a8cb7f03c3f3a5d5a6d927ed4e58eb2')
        self.assertEqual(value['expected_delta'][m.HELPER],m.prior.config()['expected_delta'][m.HELPER])

    def test_receipt_source_count_is_derived_not_stale_summary_literal(self):
        complete=json.loads((m.LATEST/'generation.json').read_text())
        self.assertEqual(m.inventory_summary(complete)['known_source_files'],21)
        smaller=dict(complete,source_sha256={k:v for k,v in list(complete['source_sha256'].items())[:2]})
        self.assertEqual(m.inventory_summary(smaller)['known_source_files'],2)
        self.assertIn("report['gates']['latest_inventory']=inventory_summary(complete)",(HERE/'run.py').read_text())

    def test_fixture_fix_is_only_explicit_value_type_before_callback(self):
        fixture=(m.SOURCE/'fixture.rs').read_text()
        original='let shared = Rc::new(RefCell::new(RetainedBook::default())); let weak = Rc::downgrade(&shared);'
        corrected='let shared: Rc<RefCell<RetainedBook<Value>>> = Rc::new(RefCell::new(RetainedBook::default())); let weak = Rc::downgrade(&shared);'
        self.assertEqual(fixture.count(corrected),1)
        self.assertEqual(m.sha(fixture.replace(corrected,original).encode()),'50803000cf238cea8d48ac925f4b52551af954c64d51650bb615418e90af54e5')

if __name__=='__main__':unittest.main()

