import ast
import contextlib
import importlib.util
import io
import json
import os
from pathlib import Path
import tempfile
from types import SimpleNamespace
import unittest
from unittest import mock

HERE=Path(__file__).resolve().parent
spec=importlib.util.spec_from_file_location('host_ci',HERE/'run.py');m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m)

class Checks(unittest.TestCase):
    def test_exact_input_files_and_names(self):
        c=m.read_inputs();m.validate_inputs(c)
        names=[n for g in c['groups'] for n in g['names']]
        self.assertEqual(len(names),18);self.assertEqual(len(set(names)),18)
        self.assertEqual(c['runtime_source'],'035273c4805d75111e528c8c26607fa3a8d1d5bf')
        c['official']['horizon']['url']='https://example.invalid/replacement.git'
        with self.assertRaises(ValueError):m.validate_inputs(c)

    def test_test_inventory_rejects_omitted_or_broad_groups(self):
        for change in ['name','filter','count']:
            c=m.read_inputs()
            if change=='name':c['groups'][0]['names'][0]='missing'
            elif change=='filter':c['groups'][0]['prefix']='pluto_'
            else:c['groups'][0]['count']=0
            with self.assertRaises(ValueError):m.validate_inputs(c)

    def test_exact_whole_manifest_counts_and_no_language_hooks(self):
        import tomllib
        c=m.read_inputs();base=m.ROOT/'pluto-lib/mixins';names=['pluto_native_creative','pluto_transport_3a','pluto_tool_rules','pluto_component_display_3a','pluto_component_bar','pluto_items_b2']
        objects=[tomllib.loads((base/n/'horizon-mixin.toml').read_text()) for n in names]
        self.assertEqual({k:sum(len(x.get(k,[])) for x in objects) for k in c['manifest_counts']},c['manifest_counts'])
        main=objects[0];mods=[x for x in main['module'] if x['name'].startswith('pluto_language')]
        self.assertEqual(len(mods),2)
        for x in mods:self.assertEqual(x['file'],'crates/horizon-client/src/client_lib.rs');self.assertEqual(x['visibility'],'pub')
        self.assertFalse(any('language' in x['id'] for key in ['injection','state'] for x in main.get(key,[])))

    def valid_plan(self):
        c=m.read_inputs()
        return {'modules':[dict(name=n,mod_id=p['mod_id'],file=p['file'],visibility=p['visibility'],source=(m.ROOT/p['path']).read_text()) for n,p in c['installed_modules'].items()]}

    def test_planned_payload_and_real_public_owner(self):
        with tempfile.TemporaryDirectory() as d:
            p=Path(d)/'plan.json';v=self.valid_plan();p.write_text(json.dumps(v));self.assertEqual(len(m.verify_plan(p,m.read_inputs())),6)
            for key,value in [('file','fake.rs'),('visibility','crate'),('mod_id','other'),('source','// wrong')]:
                altered=self.valid_plan();altered['modules'][0][key]=value;p.write_text(json.dumps(altered))
                with self.assertRaises(ValueError):m.verify_plan(p,m.read_inputs())

    def test_plan_rejects_missing_or_duplicate_module(self):
        with tempfile.TemporaryDirectory() as d:
            p=Path(d)/'plan.json'
            for rows in [self.valid_plan()['modules'][:1],self.valid_plan()['modules']*2,[]]:
                p.write_text(json.dumps({'modules':rows}))
                with self.assertRaises(ValueError):m.verify_plan(p,m.read_inputs())

    def test_real_lib_bin_compiler_targets_required(self):
        lines=['Running `rustc --crate-name '+x+' src/lib.rs`' for x in ['pluto_api','pluto_inventory_api','horizon_client_render','horizon_client_lib','horizon_client']]
        lines.append('Running `rustc --crate-name horizon_client src/main.rs`')
        m.native_host_checks('\n'.join(lines))
        for i in range(len(lines)):
            with self.assertRaises(ValueError):m.native_host_checks('\n'.join(lines[:i]+lines[i+1:]))
        with self.assertRaises(ValueError):m.native_host_checks('Fresh horizon_client')

    def test_actual_client_lib_test_compilation_required(self):
        good='Running `rustc --crate-name horizon_client_lib ../horizon-client/src/client_lib.rs --test --emit=link`'
        m.native_test_compilation(good,'horizon_client_lib','client_lib.rs')
        for s in ['Fresh horizon-client-lib',good.replace(' --test ', ' '),good.replace('client_lib.rs','mock.rs')]:
            with self.assertRaises(ValueError):m.native_test_compilation(s,'horizon_client_lib','client_lib.rs')

    def test_exact_results_reject_skips_duplicates_or_zero(self):
        good='test a ... ok\ntest b ... ok\ntest result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 40 filtered out; finished in 0.1s\n'
        m.validate_tests(good,['a','b'])
        for bad in ['',good.replace('test b','test a'),good.replace('b ... ok','b ... ignored'),good.replace('2 passed','0 passed'),good+good]:
            with self.assertRaises(ValueError):m.validate_tests(bad,['a','b'])

    def test_python_gates_check_executed_count_and_names(self):
        good=''.join(f'test_{i} (Suite) ... ok\n' for i in range(19))+'Ran 19 tests in 0.1s\nOK\n'
        m.validate_python_checks(good,19)
        for bad in [good.replace('19 tests','18 tests'),good.replace('test_0 (Suite) ... ok','test_0 (Suite) ... skipped'),good.replace('OK','FAILED'),good+good]:
            with self.assertRaises(ValueError):m.validate_python_checks(bad,19)

    def test_disk_guard_checks_root_and_rustup_mounts(self):
        for low in ['/','/rustup']:
            with mock.patch.object(m,'existing_parent',side_effect=Path),mock.patch.object(m.shutil,'disk_usage',side_effect=lambda p:SimpleNamespace(free=m.FLOOR-1 if str(p)==low else m.FLOOR+4*1024**3)):
                with self.assertRaises(RuntimeError):m.Runner(Path('/work'),{'CARGO_HOME':'/cargo','RUSTUP_HOME':'/rustup'}).guard()

    def test_process_group_absence_is_bounded_even_after_leader_exit(self):
        with mock.patch.object(m.os,'killpg',side_effect=ProcessLookupError):m.ensure_group_gone(1,'finished')
        with mock.patch.object(m.os,'killpg'),mock.patch.object(m.time,'monotonic',side_effect=[0,2]):
            with self.assertRaises(RuntimeError):m.ensure_group_gone(1,'straggler')
        source=(HERE/'run.py').read_text();self.assertIn('finally:',source);self.assertIn('ensure_group_gone(proc.pid, name)',source)

    def test_checkout_and_generated_seals_required(self):
        with mock.patch.object(m,'git',return_value='?? .cargo/config.toml'):
            with self.assertRaises(ValueError):m.assert_clean_checkout()
        source=(HERE/'run.py').read_text()
        for required in ['assert_full_seal(stage,stage_seal)','out_dirs(work/\'host-target\')!=generated','Recompilation in execution phase','stage_seal=checkout_seal(stage)','assert_seal(ROOT,original,True)']:
            self.assertIn(required,source)

    def test_scope_and_workflow_are_bounded(self):
        s=(m.ROOT/'.github/workflows/common-composite-wip.yml').read_text()
        for want in ['contents: read','private == false','persist-credentials: false',m.read_inputs()['branch']]:self.assertIn(want,s)
        for bad in ['workflow_dispatch','pull_request:','actions/cache','upload-artifact','secrets.','self-hosted','contents: write']:self.assertNotIn(bad,s)
        self.assertIn('No live Game', (HERE/'run.py').read_text())

    def test_initial_partial_verification_or_summary_failure_never_pass(self):
        with tempfile.TemporaryDirectory() as d:
            for kind in ['initial','partial','unverified','summary']:
                def execute(report):
                    if kind=='initial':raise ValueError('before setup')
                    if kind in ['partial','summary']:report['verified']=True
                    if kind=='summary':report['gates']={k:True for k in m.REQUIRED_GATES}
                target=Path(d)/('missing/file' if kind=='summary' else 'summary')
                with mock.patch.object(m,'execute',side_effect=execute),mock.patch.dict(os.environ,{'GITHUB_STEP_SUMMARY':str(target)}),contextlib.redirect_stdout(io.StringIO()) as out:
                    with self.assertRaises(SystemExit):m.main()
                r=json.loads(out.getvalue().split('FINAL_RECEIPT ',1)[1]);self.assertFalse(r['passed'])

    def test_complete_checkout_seal_catches_ignored_files_and_symlink_targets(self):
        with tempfile.TemporaryDirectory() as d:
            root=Path(d);(root/'.git').mkdir();(root/'.git/config').write_text('ignored internals')
            (root/'source.rs').write_text('source');(root/'link').symlink_to('source.rs')
            before=m.checkout_seal(root)
            (root/'.cargo').mkdir();(root/'.cargo/config.toml').write_text('[build]')
            self.assertNotEqual(m.checkout_seal(root),before)
            self.assertEqual(before['link'],{'kind':'symlink','target':'source.rs'})
            self.assertFalse(any(k.startswith('.git') for k in before))
            (root/'link').unlink();(root/'link').symlink_to('elsewhere')
            self.assertNotEqual(m.checkout_seal(root)['link'],before['link'])

    def test_official_and_mod_ignored_entries_cannot_pass(self):
        with mock.patch.object(m,'git',return_value='!! .cargo/config.toml') as read:
            with self.assertRaises(ValueError):m.assert_git_clean(Path('/official'))
            self.assertIn('--ignored=matching',read.call_args.args)
        source=(HERE/'run.py').read_text()
        for wanted in ['checkouts[str(checkout)]=checkout_seal(checkout)','Full official checkout drift','Full Mod checkout drift']:
            self.assertIn(wanted,source)


    def make_artifact(self, binary, target, stage):
        return json.dumps({'reason':'compiler-artifact','target':{'name':target['crate'],'kind':['lib'],
            'src_path':str(stage/target['source'])},'profile':{'test':True,'opt_level':'0','debug_assertions':True},
            'executable':str(binary),'fresh':False})

    def test_actual_artifact_selection_rejects_wrong_owner_duplicate_or_escape(self):
        with tempfile.TemporaryDirectory() as d:
            root=Path(d);stage=root/'stage';output=root/'target';(output/'debug/deps').mkdir(parents=True)
            binary=output/'debug/deps/test';binary.write_text('not executed');binary.chmod(0o700)
            for target in m.read_inputs()['targets'].values():
                good=self.make_artifact(binary,target,stage)
                self.assertEqual(m.select_test_artifact(good,target,stage,output),binary)
                bad=json.loads(good);bad['target']['src_path']=str(stage/'wrong.rs')
                bad2=json.loads(good);bad2['executable']=str(root/'outside')
                bad3=json.loads(good);bad3['profile']['opt_level']='3'
                for log in ['',good+'\n'+good,json.dumps(bad),json.dumps(bad2),json.dumps(bad3)]:
                    with self.assertRaises(ValueError):m.select_test_artifact(log,target,stage,output)

    def test_actual_inventory_rejects_filter_extra_duplicate_and_wrong_summary(self):
        groups=[{'prefix':'scope::','names':['scope::a','scope::b'],'count':2}]
        good='scope::a: test\nscope::b: test\nother::c: test\n\n3 tests, 0 benchmarks\n'
        self.assertEqual(m.validate_list(good,groups),3)
        for bad in [good.replace('scope::b','scope::a'),good.replace('other::c','scope::c'),good.replace('3 tests','2 tests'),'',good+good]:
            with self.assertRaises(ValueError):m.validate_list(bad,groups)

    def test_execution_uses_direct_sealed_binaries_after_both_compilations(self):
        source=(HERE/'run.py').read_text();tree=ast.parse(source)
        execute=next(n for n in tree.body if isinstance(n,ast.FunctionDef) and n.name=='execute')
        group_loop=next(n for n in ast.walk(execute) if isinstance(n,ast.For) and ast.unparse(n.target)=='group')
        calls=[n for n in ast.walk(group_loop) if isinstance(n,ast.Call) and isinstance(n.func,ast.Attribute) and n.func.attr=='run']
        self.assertEqual(len(calls),1)
        self.assertEqual(ast.unparse(calls[0].args[1]),"[str(binary), group['prefix'], '--test-threads=1']")
        self.assertIn('assert_binaries(binaries)',ast.unparse(group_loop))
        self.assertLess(source.index("gates[name+'_compile']"),source.index("generated=out_dirs(work/'host-target')"))
        self.assertIn("totals[group['target']]-group['count']",source)

    def test_compact_console_keeps_evidence_not_verbose_noise(self):
        raw='x'*250000+'\ntest required ... ok\ntest result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;\n'
        with contextlib.redirect_stdout(io.StringIO()) as out:m.print_compact_log(raw,False)
        self.assertLess(len(out.getvalue()),512);self.assertIn('test required ... ok',out.getvalue())
        with self.assertRaises(ValueError):m.print_compact_log('test '+'x'*140000,False)
        with contextlib.redirect_stdout(io.StringIO()) as out:m.print_compact_log('x'*50000,True)
        self.assertLess(len(out.getvalue()),17000);self.assertIn('FAILURE_TAIL',out.getvalue())

    def test_compiler_witnesses_keep_actual_identity_and_hash(self):
        log='Running `rustc --crate-name horizon_client_lib ../horizon-client/src/client_lib.rs --test -C debuginfo=0`\nFresh horizon-client-lib\nRunning `rustc --crate-name unrelated x.rs`'
        rows=m.compiler_records(log);self.assertEqual(len(rows),1)
        self.assertEqual(rows[0]['crate'],'horizon_client_lib');self.assertTrue(rows[0]['test'])
        self.assertEqual(rows[0]['source'],['../horizon-client/src/client_lib.rs'])
        self.assertEqual(len(rows[0]['invocation_sha256']),64)

    def test_exact_cutoff_archive_branch_and_targets_are_not_configurable(self):
        for key,value in [('repository','other/repo'),('branch','other'),('runtime_source','old'),('bundle_sha256','0'*64),('official_lock_sha256','0'*64),('manifest_counts',{'module':1}),('targets',{})]:
            c=m.read_inputs();c[key]=value
            with self.assertRaises(ValueError):m.validate_inputs(c)
        c=m.read_inputs();c['groups'].append(c['groups'][0])
        with self.assertRaises(ValueError):m.validate_inputs(c)

    def test_failed_subprocess_still_records_bounded_phase_receipt(self):
        import sys
        with tempfile.TemporaryDirectory() as d:
            report={'phases':[]};runner=m.Runner(Path(d),dict(os.environ),report)
            with mock.patch.object(runner,'guard'),contextlib.redirect_stdout(io.StringIO()) as out:
                with self.assertRaises(RuntimeError):runner.run('fails',[sys.executable,'-c','print("failure diagnostic");raise SystemExit(7)'],timeout=5)
            self.assertEqual(len(report['phases']),1);row=report['phases'][0]
            self.assertEqual(row['exit_code'],7);self.assertIsNotNone(row['error'])
            self.assertEqual(row['raw_sha256'],m.sha(b'failure diagnostic\n'))
            self.assertIn('PHASE_RECEIPT',out.getvalue())

if __name__=='__main__':unittest.main()
