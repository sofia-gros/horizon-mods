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
    def test_pinned_sources_inventory_and_only_one_changed_prior_output(self):
        value=m.config();old=m.old.config()
        self.assertEqual(len(value['inventory_names']),17);self.assertEqual(len(value['ownership_names']),10)
        self.assertEqual({p for p in value['expected_delta'] if value['expected_delta'][p]!=old['expected_delta'][p]}, {m.HELPER})
        self.assertEqual(len(value['expected_delta']),11)

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

    def test_inventory_rejects_missing_duplicate_extra_or_wrong_summary(self):
        names=m.config()['inventory_names'];good=''.join(n+': test\n' for n in names)+'17 tests, 0 benchmarks\n'
        m.inventory(good,names)
        for bad in ['',good.replace(names[0],names[1]),good+names[0]+': test\n',good.replace('17 tests','16 tests')]:
            with self.assertRaises(ValueError):m.inventory(bad,names)

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
        self.assertIn('old.prepare(pristine,experiment)',source)
        self.assertIn('target = experiment/HELPER',source)
        self.assertNotIn('target = pristine',source)
        self.assertIn('old.verify_delta(pristine,experiment',source)
        self.assertIn('if seal(pristine)!=before',source)

    def test_binary_and_all_source_seals_survive_to_receipt(self):
        source=(HERE/'run.py').read_text()
        for token in ['base.binary_seal(binary)!=compiled','if seal(ROOT)!=source','if seal(Path(name))!=before','old.out_dirs(work/\'target\')!=generated','report[\'verified\']=True']:
            self.assertIn(token,source)
        self.assertIn("'host_type'",m.REQUIRED.__repr__())

    def test_workflow_stays_read_only_dedicated_and_without_extra_services(self):
        source=(m.ROOT/'.github/workflows/guide-owned-opening-wip.yml').read_text()
        for token in ['contents: read','private == false','persist-credentials: false',m.BRANCH,'ubuntu-24.04']:
            self.assertIn(token,source)
        for token in ['workflow_dispatch','pull_request:','actions/cache','upload-artifact','secrets.','self-hosted','contents: write']:
            self.assertNotIn(token,source)
        runner=(HERE/'run.py').read_text()
        for token in ["'sudo'","'apt-get'",'horizon-mixins','package_native.py']:
            self.assertNotIn(token,runner)

if __name__=='__main__':unittest.main()
