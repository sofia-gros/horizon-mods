"""Small source-only runner checks. No Cargo, plan, apply or native subprocess."""
import ast,contextlib,importlib.util,io,json,os,tempfile,unittest
from pathlib import Path
from types import SimpleNamespace
from unittest import mock
HERE=Path(__file__).resolve().parent
spec=importlib.util.spec_from_file_location('removal_ci',HERE/'run.py')
m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m)

class Checks(unittest.TestCase):
    def test_exact_source_pins_and_complete_input_set(self):
        c=m.read_inputs();m.validate_inputs(c)
        self.assertEqual(len(c['source_sha256']),19)
        for key,value in [('repository','other/repo'),('branch','other'),('base','0'*40),('runtime_source','0'*40),('source_sha256',{})]:
            changed=m.read_inputs();changed[key]=value
            with self.assertRaises(ValueError):m.validate_inputs(changed)
        changed=m.read_inputs();changed['official']['horizon']['url']='https://example.invalid/repo.git'
        with self.assertRaises(ValueError):m.validate_inputs(changed)

    def test_native_groups_are_exact_disjoint_and_source_derived(self):
        c=m.read_inputs();self.assertEqual([len(g['tests']) for g in c['groups']],[18,8,6])
        for change in ['name','prefix','extra']:
            c=m.read_inputs()
            if change=='name':c['groups'][0]['tests'][0]='missing'
            elif change=='prefix':c['groups'][0]['prefix']='other::'
            else:c['groups'].append(c['groups'][0])
            with self.assertRaises(ValueError):m.validate_inputs(c)

    def test_push_public_repo_and_exact_branch_only(self):
        event={'repository':{'private':False}}
        env={'GITHUB_ACTIONS':'true','GITHUB_REPOSITORY':m.REPOSITORY,'GITHUB_EVENT_NAME':'push','GITHUB_REF':'refs/heads/'+m.BRANCH}
        m.assert_event(m.read_inputs(),event,env)
        for k,v in [('GITHUB_ACTIONS','false'),('GITHUB_REPOSITORY','other/repo'),('GITHUB_EVENT_NAME','workflow_dispatch'),('GITHUB_REF','refs/heads/main')]:
            with self.assertRaises(ValueError):m.assert_event({},event,dict(env,**{k:v}))
        for e in [{},{'repository':{'private':True}}]:
            with self.assertRaises(ValueError):m.assert_event({},e,env)

    def test_cli_counts_cannot_broaden_candidate(self):
        c={'status':'ok','injections':3,'modules':2,'bridges':0,'states':0,'crates':0,'data':0,'post_build':0}
        self.assertEqual(m.validate_cli(json.dumps(c))['injections'],3)
        for k in c:
            wrong=dict(c);wrong[k]='error' if k=='status' else c[k]+1
            with self.assertRaises(ValueError):m.validate_cli(json.dumps(wrong))

    def test_actual_compile_not_fresh_and_single_test(self):
        good='Running `rustc --crate-name pluto_item_name_removal_tests lib.rs --test --emit=link`'
        self.assertEqual(len(m.require_compile(good,'pluto_item_name_removal_tests',True)),1)
        for bad in ['Fresh pluto-item-name-removal-tests',good.replace('--test',''),good+'\n'+good]:
            with self.assertRaises(ValueError):m.require_compile(bad,'pluto_item_name_removal_tests',True)
        for name in ['horizon_core','horizon_client_core','horizon_items','horizon_protocol','horizon_world','horizon_mixins']:
            m.require_compile('Running `rustc --crate-name '+name+' src/lib.rs`',name)

    def test_actual_compiler_witness_preserves_source_and_digest(self):
        raw='Running `rustc --crate-name horizon_items crates/horizon-items/src/lib.rs`\nFresh horizon-items\nRunning `rustc --crate-name unrelated x.rs`'
        rows=m.compiler_records(raw);self.assertEqual(len(rows),1)
        self.assertEqual(rows[0]['source'],['crates/horizon-items/src/lib.rs']);self.assertFalse(rows[0]['test'])
        self.assertEqual(len(rows[0]['invocation_sha256']),64)

    def test_artifact_rejects_wrong_source_profile_duplicate_or_escape(self):
        with tempfile.TemporaryDirectory() as d:
            root=Path(d);fixture=root/'fixture';target=root/'target';(target/'debug/deps').mkdir(parents=True)
            binary=target/'debug/deps/test';binary.write_text('never executed');binary.chmod(0o700)
            good={'reason':'compiler-artifact','target':{'name':'pluto_item_name_removal_tests','kind':['lib'],'src_path':str(fixture/'lib.rs')},'profile':{'test':True,'opt_level':'0','debug_assertions':True},'executable':str(binary)}
            self.assertEqual(m.select_artifact(json.dumps(good),fixture,target),binary)
            for which in ['source','kind','profile','path','duplicate','empty']:
                v=json.loads(json.dumps(good))
                if which=='source':v['target']['src_path']=str(fixture/'wrong.rs')
                elif which=='kind':v['target']['kind']=['bin']
                elif which=='profile':v['profile']['opt_level']='3'
                elif which=='path':v['executable']=str(root/'outside')
                log=json.dumps(v)
                if which=='duplicate':log+='\n'+log
                if which=='empty':log=''
                with self.assertRaises(ValueError):m.select_artifact(log,fixture,target)

    def test_exact_binary_inventory(self):
        names=[n for g in m.read_inputs()['groups'] for n in g['tests']]
        good='\n'.join(n+': test' for n in names)+'\n32 tests, 0 benchmarks\n';m.validate_list(good,names)
        for bad in ['',good+good,good.replace(names[0],names[1]),good.replace('32 tests','31 tests'),good+'unexpected: test\n']:
            with self.assertRaises(ValueError):m.validate_list(bad,names)

    def test_results_reject_skips_filters_duplicates_and_zero(self):
        names=['a','b'];good='test a ... ok\ntest b ... ok\ntest result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.1s\n'
        m.validate_tests(good,names,filtered=0)
        for bad in ['',good+good,good.replace('test b','test a'),good.replace('b ... ok','b ... ignored'),good.replace('2 passed','0 passed'),good.replace('0 filtered','1 filtered'),good.replace('0 ignored','1 ignored')]:
            with self.assertRaises(ValueError):m.validate_tests(bad,names,filtered=0)

    def test_python_counts_require_actual_ok_rows(self):
        good='test_a (Suite) ... ok\ntest_b (Suite) ... ok\nRan 2 tests in 0.1s\nOK\n';m.validate_python_checks(good,2)
        for bad in ['',good+good,good.replace('2 tests','1 test'),good.replace('... ok','... skipped'),good.replace('OK','FAILED')]:
            with self.assertRaises(ValueError):m.validate_python_checks(bad,2)

    def test_each_writable_filesystem_and_startup_reserve(self):
        for low in ['/','/rustup','/cargo','/work',str(m.ROOT)]:
            with mock.patch.object(m,'existing_parent',side_effect=Path),mock.patch.object(m.shutil,'disk_usage',side_effect=lambda p:SimpleNamespace(free=m.FLOOR-1 if str(p)==low else m.FLOOR+4*1024**3)):
                with self.assertRaises(RuntimeError):m.Runner(Path('/work'),{'CARGO_HOME':'/cargo','RUSTUP_HOME':'/rustup'}).guard()
        with mock.patch.object(m,'write_filesystems',return_value=[Path('/work')]),mock.patch.object(m.shutil,'disk_usage',return_value=SimpleNamespace(free=m.FLOOR+1)):
            with self.assertRaises(RuntimeError):m.Runner(Path('/work'),{}).guard(reserve=2*1024**3)

    def test_group_teardown_even_when_leader_already_exited(self):
        with tempfile.TemporaryDirectory() as d:
            report={'phases':[]};runner=m.Runner(Path(d),{},report)
            proc=mock.Mock(pid=77,returncode=0);proc.poll.return_value=0
            with mock.patch.object(runner,'guard'),mock.patch.object(m.subprocess,'Popen',return_value=proc),mock.patch.object(m.os,'killpg') as kill,mock.patch.object(m,'ensure_group_gone') as gone,contextlib.redirect_stdout(io.StringIO()):
                runner.run('mock',['not-executed'])
            kill.assert_called_once_with(77,m.signal.SIGKILL);proc.wait.assert_called_once_with(timeout=2);gone.assert_called_once_with(77,'mock')

    def test_late_floor_violation_never_passes_phase(self):
        with tempfile.TemporaryDirectory() as d:
            report={'phases':[]};runner=m.Runner(Path(d),{},report)
            proc=mock.Mock(pid=88,returncode=0);proc.poll.return_value=0
            with mock.patch.object(runner,'guard',side_effect=[None,RuntimeError('late floor')]),mock.patch.object(m.subprocess,'Popen',return_value=proc),mock.patch.object(m.os,'killpg'),mock.patch.object(m,'ensure_group_gone'),contextlib.redirect_stdout(io.StringIO()):
                with self.assertRaises(RuntimeError):runner.run('mock',['not-executed'])
            self.assertIn('late floor',report['phases'][0]['error'])

    def test_live_child_failure_teardown_is_bounded(self):
        with tempfile.TemporaryDirectory() as d:
            runner=m.Runner(Path(d),{});proc=mock.Mock(pid=99,returncode=-9);proc.poll.return_value=None
            with mock.patch.object(runner,'guard',side_effect=[None,RuntimeError('floor')]),mock.patch.object(m.subprocess,'Popen',return_value=proc),mock.patch.object(m.os,'killpg') as kill,mock.patch.object(m,'ensure_group_gone') as gone,mock.patch.object(m.time,'monotonic',side_effect=[0,1,3,4]),contextlib.redirect_stdout(io.StringIO()):
                with self.assertRaises(RuntimeError):runner.run('mock',['not-executed'])
            self.assertEqual(kill.call_args_list,[mock.call(99,m.signal.SIGTERM),mock.call(99,m.signal.SIGKILL)])
            proc.wait.assert_called_once_with(timeout=2);gone.assert_called_once()

    def test_group_absence_check_has_deadline(self):
        with mock.patch.object(m.os,'killpg',side_effect=ProcessLookupError):m.ensure_group_gone(1,'done')
        with mock.patch.object(m.os,'killpg'),mock.patch.object(m.time,'monotonic',side_effect=[0,2]):
            with self.assertRaises(RuntimeError):m.ensure_group_gone(1,'remaining')

    def test_full_seal_includes_untracked_ignored_links_and_modes(self):
        with tempfile.TemporaryDirectory() as d:
            root=Path(d);(root/'.git').mkdir();(root/'.git/config').write_text('git internal')
            p=root/'source';p.write_text('bytes');(root/'link').symlink_to('source');before=m.checkout_seal(root)
            self.assertFalse(any(n.startswith('.git') for n in before));self.assertEqual(before['link']['target'],'source')
            p.chmod(0o700)
            with self.assertRaises(ValueError):m.assert_full_seal(root,before)
            (root/'.cargo').mkdir();(root/'.cargo/config.toml').write_text('ignored injection')
            self.assertIn('.cargo/config.toml',m.checkout_seal(root))

    def test_ignored_checkout_is_not_clean(self):
        with mock.patch.object(m,'git',return_value='!! .cargo/config.toml') as g:
            with self.assertRaises(ValueError):m.assert_git_clean(Path('/official'))
            self.assertIn('--ignored=matching',g.call_args.args)

    def test_generated_source_drift_and_links_are_detected(self):
        with tempfile.TemporaryDirectory() as d:
            target=Path(d);out=target/'debug/build/example/out';out.mkdir(parents=True)
            (out/'generated.rs').write_text('one');before=m.output_seal(target)
            (out/'generated.rs').write_text('two');self.assertNotEqual(before,m.output_seal(target))
            (out/'link').symlink_to('generated.rs')
            with self.assertRaises(ValueError):m.output_seal(target)

    def test_plan_guard_apply_extraction_and_direct_execution_order(self):
        s=(HERE/'run.py').read_text()
        ordered=["runner.run('actual-plan'","runner.run('exclusive-guard'","runner.run('actual-apply'","runner.run('prepare-applied-regions'","runner.run('metadata-resolve'","runner.run('native-codec-compile'","runner.run('native-inventory'","runner.run('native32'"]
        self.assertEqual([s.index(x) for x in ordered],sorted(s.index(x) for x in ordered))
        self.assertIn("[str(binary),'--test-threads=1']",s);self.assertIn('generated=output_seal',s)
        self.assertIn('assert_full_seal(stage,stage_seal)',s);self.assertIn('assert_full_seal(fixture,fixture_seal)',s)
        self.assertIn("output_seal(work/'target')!=generated",s);self.assertIn('binary.read_bytes())!=binary_sha',s)

    def test_workflow_is_exact_public_push_read_only(self):
        s=(m.ROOT/'.github/workflows/item-name-removal-wip.yml').read_text()
        for text in [m.BRANCH,'contents: read','private == false','persist-credentials: false','ubuntu-24.04',"github.event_name == 'push'"]:self.assertIn(text,s)
        for text in ['workflow_dispatch','pull_request:','secrets.','self-hosted','actions/cache','upload-artifact','contents: write']:self.assertNotIn(text,s)

    def test_cold_fetch_then_offline_and_no_installed_package(self):
        s=(HERE/'run.py').read_text()
        self.assertLess(s.index("'-locked-fetch'"),s.index("env['CARGO_NET_OFFLINE']='true'"))
        self.assertIn("'--locked','--offline'",s)
        self.assertIn("candidate=BASE/'candidate'",s)
        self.assertNotIn('package_native.py',s);self.assertNotIn("'-p','horizon-client'",s)
        self.assertIn('Not installed common package',s)

    def test_initial_missing_gates_verification_or_summary_fail_closed(self):
        with tempfile.TemporaryDirectory() as d:
            for kind in ['initial','missing','unverified','summary']:
                def execute(report):
                    if kind=='initial':raise ValueError('before setup')
                    if kind in ['missing','summary']:report['verified']=True
                    if kind=='summary':report['gates']={k:True for k in m.REQUIRED_GATES}
                target=Path(d)/('missing/file' if kind=='summary' else 'summary')
                with mock.patch.object(m,'execute',side_effect=execute),mock.patch.dict(os.environ,{'GITHUB_STEP_SUMMARY':str(target)}),contextlib.redirect_stdout(io.StringIO()) as out:
                    with self.assertRaises(SystemExit):m.main()
                self.assertFalse(json.loads(out.getvalue().split('FINAL_RECEIPT ',1)[1])['passed'])

    def test_console_retains_required_evidence_not_raw_noise(self):
        raw='x'*250000+'\ntest required ... ok\ntest result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;\n'
        with contextlib.redirect_stdout(io.StringIO()) as out:m.print_compact_log(raw,False)
        self.assertLess(len(out.getvalue()),512);self.assertIn('test required ... ok',out.getvalue())
        with self.assertRaises(ValueError):m.print_compact_log('test '+'x'*140000,False)
        with contextlib.redirect_stdout(io.StringIO()) as out:m.print_compact_log('x'*50000,True)
        self.assertLess(len(out.getvalue()),17000)

if __name__=='__main__':unittest.main()
