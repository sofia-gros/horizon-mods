"""Small source-only runner checks. No Cargo, plan, apply or native subprocess."""
import ast,contextlib,importlib.util,io,json,os,tempfile,unittest
from pathlib import Path
from types import SimpleNamespace
from unittest import mock
HERE=Path(__file__).resolve().parent
spec=importlib.util.spec_from_file_location('crafting_stored_ci',HERE/'run.py')
m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m)

class Checks(unittest.TestCase):
    def test_exact_source_pins_and_complete_input_set(self):
        c=m.read_inputs();m.validate_inputs(c)
        self.assertEqual(len(c['source_sha256']),52)
        for key,value in [('repository','other/repo'),('branch','other'),('base','0'*40),('source_sha256',{})]:
            changed=m.read_inputs();changed[key]=value
            with self.assertRaises(ValueError):m.validate_inputs(changed)
        changed=m.read_inputs();changed['official']['url']='https://example.invalid/repo.git'
        with self.assertRaises(ValueError):m.validate_inputs(changed)

    def test_native_groups_are_exact_disjoint_and_source_derived(self):
        c=m.read_inputs();self.assertEqual([g['count'] for g in c['groups']],[153,16])
        self.assertEqual([len(c['test_groups'][k]) for k in ['api_existing147','api_added6']],[147,6])
        for change in ['name','package','extra','subgroup']:
            c=m.read_inputs()
            if change=='name':c['tests']['api153'][0]='missing'
            elif change=='package':c['groups'][0]['package']='other'
            elif change=='subgroup':c['test_groups']['api_added6']=[]
            else:c['groups'].append(c['groups'][0])
            with self.assertRaises(ValueError):m.validate_inputs(c)

    def test_push_public_repo_and_exact_branch_only(self):
        event={'repository':{'private':False}}
        env={'GITHUB_ACTIONS':'true','GITHUB_REPOSITORY':m.REPOSITORY,'GITHUB_EVENT_NAME':'push','GITHUB_REF':'refs/heads/'+m.BRANCH}
        m.assert_event(m.read_inputs(),event,env)
        for k,v in [('GITHUB_ACTIONS','false'),('GITHUB_REPOSITORY','other/repo'),('GITHUB_EVENT_NAME','workflow_dispatch'),('GITHUB_REF','refs/heads/main'),('GITHUB_REF','refs/heads/integration/tinkers-latest-playtest-20261106')]:
            with self.assertRaises(ValueError):m.assert_event({},event,dict(env,**{k:v}))
        workflow=(m.ROOT/m.WORKFLOW).read_text()
        self.assertIn('      - '+m.BRANCH,workflow)
        self.assertIn('contents: read',workflow)
        self.assertNotIn('workflow_dispatch',workflow)
        self.assertNotIn('pull_request:',workflow)
        self.assertNotIn('integration/tinkers',workflow)
        self.assertIn('persist-credentials: false',workflow)
        for e in [{},{'repository':{'private':True}}]:
            with self.assertRaises(ValueError):m.assert_event({},e,env)

    def test_compile_identity_rejects_wrong_source(self):
        good='Running `rustc --crate-name horizon_items /official/crates/horizon-items/src/lib.rs`'
        m.require_compile(good,'horizon_items',Path('/official/crates/horizon-items/src/lib.rs'),Path('/fixture'))
        with self.assertRaises(ValueError):m.require_compile(good,'horizon_items',Path('/other/lib.rs'),Path('/fixture'))

    def test_actual_compile_not_fresh_and_single_test(self):
        good='Running `rustc --crate-name pluto_crafting_stored_tests lib.rs --test --emit=link`'
        self.assertEqual(len(m.require_compile(good,'pluto_crafting_stored_tests',Path('/fixture/lib.rs'),Path('/fixture'),True)),1)
        for bad in ['Fresh pluto-latest-template-tests',good.replace('--test',''),good+'\n'+good]:
            with self.assertRaises(ValueError):m.require_compile(bad,'pluto_crafting_stored_tests',Path('/fixture/lib.rs'),Path('/fixture'),True)

    def test_actual_compiler_witness_preserves_source_and_digest(self):
        raw='Running `rustc --crate-name horizon_items crates/horizon-items/src/lib.rs`\nFresh horizon-items\nRunning `rustc --crate-name unrelated x.rs`'
        rows=m.compiler_records(raw);self.assertEqual(len(rows),1)
        self.assertEqual(rows[0]['source'],['crates/horizon-items/src/lib.rs']);self.assertFalse(rows[0]['test'])
        self.assertEqual(len(rows[0]['invocation_sha256']),64)

    def test_artifact_rejects_wrong_source_profile_duplicate_or_escape(self):
        with tempfile.TemporaryDirectory() as d:
            root=Path(d);fixture=root/'fixture';target=root/'target';(target/'debug/deps').mkdir(parents=True)
            binary=target/'debug/deps/test';binary.write_text('never executed');binary.chmod(0o700)
            good={'package_id':'fixture-id','reason':'compiler-artifact','fresh':False,'target':{'name':'pluto_crafting_stored_tests','kind':['lib'],'src_path':str(fixture/'lib.rs')},'profile':{'test':True,'opt_level':'0','debug_assertions':True},'executable':str(binary)}
            self.assertEqual(m.select_artifact([good],fixture/'lib.rs',target,'pluto_crafting_stored_tests','fixture-id'),binary)
            for which in ['source','kind','profile','path','duplicate','empty','fresh','package']:
                v=json.loads(json.dumps(good))
                if which=='package':v['package_id']='other-id'
                elif which=='fresh':v['fresh']=True
                elif which=='source':v['target']['src_path']=str(fixture/'wrong.rs')
                elif which=='kind':v['target']['kind']=['bin']
                elif which=='profile':v['profile']['opt_level']='3'
                elif which=='path':v['executable']=str(root/'outside')
                log=[v]
                if which=='duplicate':log+=log
                if which=='empty':log=[]
                with self.assertRaises(ValueError):m.select_artifact(log,fixture/'lib.rs',target,'pluto_crafting_stored_tests','fixture-id')

    def test_exact_binary_inventory(self):
        c=m.read_inputs()
        for group in c['groups']:
            names=c['tests'][group['name']];count=str(len(names))
            good='\n'.join(n+': test' for n in names)+'\n'+count+' tests, 0 benchmarks\n';m.validate_list(good,names)
            for bad in ['',good+good,good.replace(names[0],names[1]),good.replace(count+' tests','0 tests'),good+'unexpected: test\n']:
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

    def test_compile_whole_inventory_before_only_direct_execution(self):
        s=(HERE/'run.py').read_text()
        self.assertLess(s.index("runner.run('workspace-metadata'"),s.index("runner.run(name+'-compile'"))
        self.assertLess(s.index("runner.run(name+'-compile'"),s.index("runner.run(name+'-inventory'"))
        self.assertLess(s.index("runner.run(name+'-inventory'"),s.index("runner.run(name+'-tests'"))
        after=s[s.index('# No Cargo follows this point:'):]
        self.assertNotIn("['cargo'",after)
        self.assertIn("[str(binary),'--test-threads=1']",after)
        self.assertIn("len({path for path,digest in binary_seals.values()})!=2",s)
        self.assertIn("output_seal(work/'target')!=generated",s)
        self.assertIn('assert_full_seal(workspace,fixture_seal)',s)
        self.assertIn("binary.read_bytes())!=digest",s)

    def test_only_pinned_workflow_and_no_trigger_creation(self):
        self.assertEqual([n for n in m.read_inputs()['source_sha256'] if n.startswith('.github/')],[m.WORKFLOW])
        s=(HERE/'run.py').read_text()
        self.assertNotIn('workflow_dispatch',s)
        for text in ['api153','finite16','bootstrap','production activation']:self.assertIn(text,s)

    def test_cold_fetch_then_offline_and_no_installed_package(self):
        s=(HERE/'run.py').read_text()
        self.assertLess(s.index("runner.run('official-locked-fetch'"),s.index("env['CARGO_NET_OFFLINE']='true'"))
        self.assertIn("'--locked','--offline'",s)
        for forbidden in ['package_native.py',"'-p','horizon-client'","'actual-plan'","'actual-apply'","'mixins-build'"]:
            self.assertNotIn(forbidden,s)
        self.assertIn('No Mixins plan/apply, constructor/bootstrap',s)

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

    def test_preparer_subprocess_cannot_escape_outer_group(self):
        s=(HERE/'run.py').read_text();source=(m.PREPARER/'prepare.py').read_text()
        self.assertIn("PLUTO_FIXTURE_CONTROLLER='crafting-stored-v1'",s)
        self.assertIn('start_new_session=False',source);self.assertNotIn('start_new_session=True',source)
        self.assertNotIn('os.killpg(',source);self.assertIn('os.getpgrp()!=os.getpid()',source)
        self.assertIn('p.kill()',source);self.assertIn('p.wait(timeout=2)',source)

    def test_second_compile_may_add_but_not_replace_existing_outputs(self):
        m.assert_old_outputs_unchanged({'a':{'x':'hash'}},{'a':{'x':'hash'},'b':{}})
        for after in [{},{'a':{'x':'changed'}}]:
            with self.assertRaises(ValueError):m.assert_old_outputs_unchanged({'a':{'x':'hash'}},after)

    def test_final_log_read_is_bounded_and_stable(self):
        with tempfile.TemporaryDirectory() as d:
            p=Path(d)/'phase';p.write_bytes(b'abc')
            self.assertEqual(m.read_log(p),b'abc')
            with mock.patch.object(m,'LOG_LIMIT',2),mock.patch.object(Path,'open',side_effect=AssertionError('must reject before open')):
                with self.assertRaises(ValueError):m.read_log(p)
            with mock.patch.object(Path,'open',return_value=io.BytesIO(b'grown')):
                with self.assertRaises(ValueError):m.read_log(p)

    def test_raw_log_failure_keeps_false_phase_receipt(self):
        with tempfile.TemporaryDirectory() as d:
            report={'phases':[]};runner=m.Runner(Path(d),{},report)
            proc=mock.Mock(pid=77,returncode=0);proc.poll.return_value=0
            with mock.patch.object(runner,'guard'),mock.patch.object(m.subprocess,'Popen',return_value=proc),mock.patch.object(m.os,'killpg'),mock.patch.object(m,'ensure_group_gone'),mock.patch.object(m,'read_log',side_effect=ValueError('too large')),contextlib.redirect_stdout(io.StringIO()):
                with self.assertRaises(ValueError):runner.run('mock',['not-executed'])
            self.assertIsNone(report['phases'][0]['raw_sha256']);self.assertIn('too large',report['phases'][0]['raw_read_error'])

    def test_artifact_stdout_accepts_only_exact_package_notices(self):
        packages=[{'id':'package-id','name':'some-crate','version':'1.2.3','targets':[{'kind':['custom-build']}]}]
        finish=json.dumps({'reason':'build-finished','success':True})
        good='[some-crate 1.2.3] cargo:warning=example\n'+finish+'\n'
        rows,notices=m.artifact_rows(good,packages);self.assertEqual(len(rows),1);self.assertEqual(notices,{'[some-crate 1.2.3] ':1})
        for bad in ['noise\n'+finish,good.replace('1.2.3','1.2.4'),good.replace('[some-crate','[other'),'{broken\n'+finish,finish+'\n'+finish,finish.replace('true','false'),json.dumps({'reason':'compiler-artifact','package_id':'unknown'})+'\n'+finish,'{"reason":"build-finished","success":true,"success":true}', '{"reason":"build-finished","success":true,"bad":NaN}', '[1,2]\n'+finish, '{"reason":"build-finished","success":true,"nested":{"bad":1e999}}', '{"reason":"build-finished","success":true,"bad":-1e999}']:
            with self.assertRaises(ValueError):m.artifact_rows(bad,packages)
        for wrong in [[dict(packages[0],targets=[{'kind':['lib']}])],packages+[dict(packages[0],id='second')]]:
            with self.assertRaises(ValueError):m.artifact_rows(good,wrong)

    def test_metadata_versions_come_from_same_resolved_packages(self):
        p={'name':'some-crate','version':'1.2.3','id':'id','manifest_path':'/real/Cargo.toml','source':None}
        evidence={'metadata_packages':[{k:v for k,v in p.items() if k!='version'}]}
        self.assertEqual(m.validate_metadata_receipt({'packages':[p]},evidence),[p])
        for key,value in [('id','other'),('name','other'),('manifest_path','/wrong'),('version','bad] prefix')]:
            wrong=dict(p,**{key:value})
            with self.assertRaises(ValueError):m.validate_metadata_receipt({'packages':[wrong]},evidence)

    def test_compile_streams_are_separate_and_witness_uses_stderr(self):
        s=(HERE/'run.py').read_text()
        self.assertIn("stdout,stderr=runner.run(name+'-compile'",s)
        self.assertIn('rows,notices=artifact_rows(stdout,packages)',s)
        self.assertIn("require_compile(stderr,group['crate']",s)
        self.assertIn('separate_stderr=True',s)
        self.assertNotIn('except json.JSONDecodeError:continue',s[s.index('def artifact_rows('):s.index('def validate_list(')])

    def test_compiler_witness_ignores_env_and_rejects_ambiguous_commands(self):
        good='Running `CARGO_PKG_HOMEPAGE=https://serde.rs RUSTC=/tool/bin/rustc /tool/bin/rustc --crate-name horizon_items /official/items/src/lib.rs`'
        self.assertEqual(m.compiler_records(good)[0]['source'],['/official/items/src/lib.rs'])
        self.assertEqual(m.compiler_records('Running `RUSTC=/tool/bin/rustc /target/build-script`'),[])
        for bad in [good.replace('--crate-name','rustc --crate-name'),good.replace('/tool/bin/rustc --crate-name','wrapper /tool/bin/rustc --crate-name'),good.replace('--crate-name horizon_items','--crate-name horizon_items --crate-name other')]:
            with self.assertRaises(ValueError):m.compiler_records(bad)

if __name__=='__main__':unittest.main()


