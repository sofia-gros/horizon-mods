import contextlib
import importlib.util
import io
import json
import os
from pathlib import Path
import tempfile
import unittest
from unittest import mock
HERE=Path(__file__).resolve().parent
spec=importlib.util.spec_from_file_location('cpu_ci',HERE/'run.py');m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m)
def artifact(path,stage):
    return {'reason':'compiler-artifact','target':{'name':'horizon_client_render','kind':['lib'],
        'src_path':str(stage/'crates/horizon-client-render/src/lib.rs')},
        'profile':{'test':True,'opt_level':'0','debug_assertions':True},'executable':str(path)}
def cpu_log(mode):
    labels=['no-provider'] if mode=='no-provider' else ['registered-ordinary','provider-no-claim','claimed-mixed','secondary-material']
    rows=[{'case':label,'profile_label':m.PROFILE,'debug_assertions':True,'iterations_per_sample':64,
        'warmup':8,'samples':7,'baseline_ns_per_section':[2]*7,'adapter_ns_per_section':[3]*7,'median_ratio':1.5} for label in labels]
    lines=['test '+m.TEST+' ... ok','test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 10 filtered out; finished in 0.1s']
    lines += ['PLUTO_TERRAIN_CPU '+json.dumps(row) for row in rows]
    lines += ['PLUTO_TERRAIN_REPLAY '+json.dumps({'valid_claim_resumed':mode=='registered'})]
    if mode=='registered':
        lines += ['PLUTO_TERRAIN_CASE '+s for s in ['no-claim passed 4 lighting combinations','claimed passed 4 lighting combinations','invalid-cell passed 4 lighting combinations','post-stale passed 4 lighting combinations','materials passed: layers, mixed faces, tint, Top, UV, fallback replay, Fusion full-layout smoke']]
    lines += ['PLUTO_TERRAIN_MODE '+mode+' complete; actual native CPU mesh, original synthetic resources only']
    return '\n'.join(lines)+'\n'
class Checks(unittest.TestCase):
    def test_workflow_exact_public_readonly(self):
        s=(m.ROOT/'.github/workflows/terrain-material-cpu-wip.yml').read_text()
        for token in ['- '+m.config()['branch'],'contents: read','private == false','persist-credentials: false','ubuntu-24.04']:
            self.assertIn(token,s)
        for token in ['workflow_dispatch','pull_request:','actions/cache','upload-artifact','secrets.','self-hosted']:
            self.assertNotIn(token,s)
    def test_official_allowlist_precedes_fetch(self):
        import copy
        c=m.config();m.validate_config(c)
        for key,value in [('url','https://github.com/forbidden/engine.git'),('revision','0'*40)]:
            bad=copy.deepcopy(c);bad['official']['horizon'][key]=value
            with self.assertRaises(ValueError):m.validate_config(bad)
        bad=copy.deepcopy(c);bad['official']['extra']=bad['official']['horizon']
        with self.assertRaises(ValueError):m.validate_config(bad)
        s=(HERE/'run.py').read_text()
        self.assertLess(s.index('validate_config(c);verify_sources(c)'),s.index("runner.run(name+'-fetch'"))
    def test_complete_seal_sees_ignored_files_and_link_target(self):
        with tempfile.TemporaryDirectory() as d:
            root=Path(d);(root/'source').write_text('original');(root/'link').symlink_to('source')
            baseline=m.strict_seal(root)
            self.assertEqual(baseline['link'],{'kind':'symlink','target':'source'})
            (root/'.cargo').mkdir();extra=root/'.cargo/config.toml';extra.write_text('[build]')
            with self.assertRaises(ValueError):m.assert_strict(root,baseline)
            extra.unlink();(root/'link').unlink();(root/'link').symlink_to('other')
            with self.assertRaises(ValueError):m.assert_strict(root,baseline)
    def test_ignored_checkout_fails_admission(self):
        with mock.patch.object(m.A,'git',return_value='!! .cargo/config.toml') as git:
            with self.assertRaises(ValueError):m.assert_checkout()
            self.assertIn('--ignored=matching',git.call_args.args)
    def test_frozen_inputs_and_inherited_runner(self):
        c=m.config();m.verify_sources(c)
        self.assertEqual(c['bundle_sha256'],'cd6de18cacbe7c40218ec817671bf3842aef3be0d586c613bb788babafbd3a76')
        self.assertEqual(c['production_head'],'43f174273cd5f1b9cf3acd1d26abab8838025510')
        self.assertIn('pluto-lib/experiments/terrain-author-ci/run.py',c['source_sha256'])
        self.assertEqual(len([n for n in dir(Checks) if n.startswith('test_')]),m.RUNNER_TEST_COUNT)
    def test_actual_artifact_and_duplicate(self):
        with tempfile.TemporaryDirectory() as d:
            root=Path(d);target=root/'target';exe=target/'debug/deps/actual';exe.parent.mkdir(parents=True);exe.write_bytes(b'unexecuted');exe.chmod(0o700)
            log=json.dumps(artifact(exe,root/'stage'))
            self.assertEqual(m.artifact(log,root/'stage',target),exe)
            for bad in [log+'\n'+log,'Fresh horizon-client-render','{}']:
                with self.assertRaises(ValueError):m.artifact(bad,root/'stage',target)
    def test_artifact_profile_source_and_path_fail_closed(self):
        with tempfile.TemporaryDirectory() as d:
            root=Path(d);target=root/'target';exe=target/'debug/deps/actual';exe.parent.mkdir(parents=True);exe.write_bytes(b'unexecuted');exe.chmod(0o700)
            for field,value in [('opt_level','1'),('debug_assertions',False)]:
                a=artifact(exe,root/'stage');a['profile'][field]=value
                with self.assertRaises(ValueError):m.artifact(json.dumps(a),root/'stage',target)
            a=artifact(exe,root/'foreign')
            with self.assertRaises(ValueError):m.artifact(json.dumps(a),root/'stage',target)
            a=artifact(root/'outside',root/'stage')
            with self.assertRaises(ValueError):m.artifact(json.dumps(a),root/'stage',target)
    def test_compiler_evidence_is_scoped_not_fresh(self):
        log='Fresh horizon_client_render\nRunning `rustc --crate-name horizon_client_render src/lib.rs --test -C opt-level=0`\nRunning `rustc --crate-name unrelated src/lib.rs`'
        rows=m.compiler_records(log);self.assertEqual(len(rows),1)
        self.assertTrue(rows[0]['test']);self.assertEqual(rows[0]['source'],['src/lib.rs']);self.assertEqual(rows[0]['codegen'],['opt-level=0'])
    def test_cpu_exact_counts_modes_timing(self):
        for mode in ['no-provider','registered']:
            out=cpu_log(mode);self.assertEqual(m.validate_cpu(out,mode)['exact_tests'],1)
            for bad in [out.replace(' ... ok',' ... ignored',1),out.replace('1 passed','0 passed'),out.replace('median_ratio": 1.5','median_ratio": 0.5'),out.replace('iterations_per_sample": 64','iterations_per_sample": 0'),out.replace('PLUTO_TERRAIN_MODE '+mode,'PLUTO_TERRAIN_MODE foreign')]:
                with self.assertRaises(ValueError):m.validate_cpu(bad,mode)
    def test_cpu_missing_cases_and_replay_reject(self):
        out=cpu_log('registered')
        for bad in ['\n'.join(l for l in out.splitlines() if not l.startswith('PLUTO_TERRAIN_REPLAY')),out.replace('materials passed:','wrong passed:'),out.replace('valid_claim_resumed": true','valid_claim_resumed": false'),out+'PLUTO_TERRAIN_CASE claimed passed 4 lighting combinations\n']:
            with self.assertRaises(ValueError):m.validate_cpu(bad,'registered')
    def test_failure_receipt_keeps_partial_gates(self):
        def fail(report):report['gates']['plan']='passed';raise ValueError('apply rejected')
        with tempfile.TemporaryDirectory() as d, mock.patch.object(m,'execute',side_effect=fail),mock.patch.dict(os.environ,{'GITHUB_STEP_SUMMARY':str(Path(d)/'summary')}):
            output=io.StringIO()
            with contextlib.redirect_stdout(output),self.assertRaises(SystemExit):m.main()
            receipt=json.loads(output.getvalue().split('FINAL_RECEIPT ',1)[1]);self.assertFalse(receipt['passed']);self.assertEqual(receipt['gates'],{'plan':'passed'});self.assertIn('apply rejected',receipt['error'])
    def test_missing_required_gates_cannot_pass(self):
        def incomplete(report):report['verified']=True
        with tempfile.TemporaryDirectory() as d,mock.patch.object(m,'execute',side_effect=incomplete),mock.patch.dict(os.environ,{'GITHUB_STEP_SUMMARY':str(Path(d)/'summary')}):
            output=io.StringIO()
            with contextlib.redirect_stdout(output),self.assertRaises(SystemExit):m.main()
            self.assertIn('Missing/unexpected required gate',output.getvalue())
    def test_summary_keeps_raw_digest_and_failure_tail(self):
        with tempfile.TemporaryDirectory() as d:
            work=Path(d);report={'phases':[]};r=m.Runner(work,{},report)
            def fail(*a,**k):(work/'failed.log').write_text('raw error\n');raise RuntimeError('failure')
            output=io.StringIO()
            with mock.patch.object(m.A.Runner,'run',side_effect=fail),contextlib.redirect_stdout(output),self.assertRaises(RuntimeError):r.run('failed',['unexecuted'])
            self.assertEqual(report['phases'][0]['raw_bytes'],10);self.assertEqual(report['phases'][0]['terminal'],'error');self.assertIn('FAILURE_TAIL',output.getvalue())
    def test_execution_uses_sealed_binary_not_cargo(self):
        s=(HERE/'run.py').read_text()
        self.assertIn("[str(binary),TEST,'--exact','--show-output','--test-threads=1']",s)
        self.assertIn("for mode in ['no-provider','registered']",s)
        self.assertIn('A.sha(binary.read_bytes())!=binary_hash',s)
        self.assertNotIn("testcmd+",s)
        self.assertLess(s.index("generated=A.out_dirs"),s.index("runner.run('native-material2'"))
    def test_guards_setup_and_final_seals_order(self):
        s=(HERE/'run.py').read_text()
        self.assertLess(s.index("runner.run('runner15'"),s.index("runner.run('apt-update'"))
        self.assertLess(s.index("runner.run('all12guards'"),s.index("runner.run('official-apply'"))
        self.assertLess(s.index("runner.run('fixture-plan-guard'"),s.index("runner.run('official-apply'"))
        for token in ['assert_checkout()','runner.guard(reserve=2*1024**3)',"A.out_dirs(work/'host-target')!=generated","'--verify'",'assert_strict(ROOT,before)']:
            self.assertIn(token,s)
if __name__=='__main__':unittest.main()
