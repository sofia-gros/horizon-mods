"""Pure parser/source regressions. No Git fetch, metadata, Rust or native job."""
import copy
import contextlib
import io
import importlib.util
import json
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
HERE=Path(__file__).resolve().parent
spec=importlib.util.spec_from_file_location('font_generation_ci',HERE/'run.py')
m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m)

def graph():
    return {'packages':[{'name':name,'id':name,'manifest_path':str(root/'Cargo.toml'),'targets':[{'kind':['lib'],'src_path':str(root/'lib.rs')}]} for name,root in [(n,Path('/official')/n) for n in m.NATIVE]+[(m.NAME,Path('/fixture'))]]}
def compiler_log(meta):
    return '\n'.join('Running `rustc --crate-name '+p['name'].replace('-','_')+' '+p['targets'][0]['src_path']+(' --test' if p['name']==m.NAME else '')+'`' for p in meta['packages'])
def artifact_row(meta):
    p=meta['packages'][-1]
    return {'fresh':False,'reason':'compiler-artifact','package_id':p['id'],'profile':{'test':True},'executable':'/target/debug/deps/test-binary','target':p['targets'][0]}
def results(names):
    return 'running 6 tests\n'+''.join('test '+n+' ... ok\n' for n in names)+'\ntest result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 15 filtered out; finished in 0.01s\n'

class Checks(unittest.TestCase):
    def test_frozen_controller_and_actual_inputs(self):
        value,p=m.config();self.assertEqual(value['base'],m.BASE);self.assertEqual(len(p['api_sha256']),29)

    def test_workspace_diagnostic_is_bounded_and_does_not_admit(self):
        metadata={'packages':[{'id':'api','name':'pluto-inventory-api','manifest_path':'/fixture/inventory-api/Cargo.toml'}],
                  'workspace_members':['api'],'workspace_default_members':['api'],'workspace_root':'/fixture'}
        before=copy.deepcopy(metadata);detail=m.workspace_diagnostic(metadata)
        self.assertEqual(detail['members'][0],{'id':'api','name':'pluto-inventory-api','manifest_path':'/fixture/inventory-api/Cargo.toml'})
        self.assertFalse(detail['truncated']);self.assertEqual(metadata,before)
        with self.assertRaises(ValueError):m.prep.verify_workspace(Path('/fixture'),metadata)
        metadata['workspace_members']=['x'*1000]*100;metadata['workspace_root']='y'*1000
        detail=m.workspace_diagnostic(metadata)
        self.assertTrue(detail['truncated']);self.assertEqual(detail['member_count'],100)
        self.assertEqual(len(detail['members']),8);self.assertLess(len(json.dumps(detail)),8000)
        source=(HERE/'run.py').read_text()
        self.assertLess(source.index("print('WORKSPACE_METADATA'"),source.index('prep.verify(fixture,metadata)'))

    def test_public_exact_push_gate(self):
        env={'GITHUB_ACTIONS':'true','GITHUB_REPOSITORY':m.REPOSITORY,'GITHUB_EVENT_NAME':'push','GITHUB_REF':'refs/heads/'+m.BRANCH,'GITHUB_SHA':'head'}
        event={'repository':{'private':False,'full_name':m.REPOSITORY}}
        m.event(env,event,'head')
        for key in env:
            altered=dict(env);altered[key]='other'
            with self.assertRaises(ValueError):m.event(altered,event,'head')
        event['repository']['private']=True
        with self.assertRaises(ValueError):m.event(env,event,'head')
    def test_complete_terse_inventory(self):
        names=['case'+str(i) for i in range(21)];log=''.join(n+': test\n' for n in sorted(names))
        m.inventory(log,names)
        for changed in [log+log,log+'\n',log+'21 tests, 0 benchmarks\n',log.replace(': test',': benchmark',1),log.split('\n',1)[1],log+'extra: test\n']:
            with self.assertRaises(ValueError):m.inventory(changed,names)
    def test_inventory_expected_duplicates_reject(self):
        names=['same']*21
        with self.assertRaises(ValueError):m.inventory('same: test\n'*21,names)
    def test_six_names_and_fifteen_filtered_exact(self):
        names=['case'+str(i) for i in range(6)];log=results(names);m.tests(log,names)
        for changed in [log.replace('15 filtered','14 filtered'),log.replace(' ... ok',' ... ignored',1),log+log,log.replace('test case1 ... ok\n',''),log.replace('case0','other')]:
            with self.assertRaises(ValueError):m.tests(changed,names)
    def test_actual_three_compiler_sources(self):
        meta=graph();rows=m.compilers(compiler_log(meta),meta,Path('/fixture'));self.assertEqual(len(rows),3);self.assertEqual(sum(r['test'] for r in rows),1)
    def test_compile_wrong_paths_profile_missing_duplicate_reject(self):
        meta=graph();log=compiler_log(meta)
        for changed in ['',log+'\n'+log,log.replace('/fixture/lib.rs','/wrong.rs'),log.replace(' --test',''),log.replace('/official/horizon-client-core/lib.rs','/fixture/lib.rs')]:
            with contextlib.redirect_stdout(io.StringIO()),self.assertRaises(ValueError):m.compilers(changed,meta,Path('/fixture'))
    def test_environment_rs_suffix_is_not_a_native_source(self):
        meta=graph();log=compiler_log(meta)
        changed=log.replace('Running `',"Running `CARGO_PKG_HOMEPAGE=https://serde.rs CARGO_PKG_DESCRIPTION='ends with .rs' ")
        self.assertEqual(len(m.compilers(changed,meta,Path('/fixture'))),3)
    def test_rustc_environment_is_not_an_executable(self):
        meta=graph();log=compiler_log(meta)
        changed=log.replace('Running `','Running `RUSTC=/toolchain/bin/rustc ')
        self.assertEqual(len(m.compilers(changed,meta,Path('/fixture'))),3)
        self.assertIsNone(m.compiler_arguments('Running `RUSTC=/toolchain/bin/rustc /target/build-script`'))
        with self.assertRaises(ValueError):m.compiler_arguments('Running `RUSTC=/toolchain/bin/rustc /other --crate-name core source.rs`')
    def test_duplicate_command_and_bounded_diagnostic(self):
        for line in ['Running `rustc /other/rustc --crate-name core source.rs`',
                     'Running `rustc --crate-name core --crate-name other source.rs`']:
            with self.assertRaises(ValueError):m.compilers(line,graph(),Path('/fixture'))
        log=compiler_log(graph()).replace('/fixture/lib.rs','/'+('x'*1000)+'.rs');output=io.StringIO()
        with contextlib.redirect_stdout(output),self.assertRaises(ValueError):m.compilers(log,graph(),Path('/fixture'))
        self.assertIn('COMPILER_SOURCE_MISMATCH',output.getvalue());self.assertLess(len(output.getvalue()),1000)
        self.assertNotIn('CARGO_PKG_HOMEPAGE',output.getvalue())
    def test_verbose_suite_obeys_inherited_parent_parser(self):
        names=unittest.defaultTestLoader.getTestCaseNames(Checks)
        names.remove('test_verbose_suite_obeys_inherited_parent_parser');self.assertEqual(len(names),20)
        output=io.StringIO()
        with contextlib.redirect_stdout(output),contextlib.redirect_stderr(output):
            result=unittest.TextTestRunner(stream=output,verbosity=2).run(unittest.TestSuite(Checks(n) for n in names))
        self.assertTrue(result.wasSuccessful(),output.getvalue());self.assertEqual(result.testsRun,20)
        self.assertNotIn('COMPILER_SOURCE_MISMATCH',output.getvalue())
        m.old.python_pass(output.getvalue(),20)
    def test_actual_single_test_artifact(self):
        meta=graph();row=artifact_row(meta)
        self.assertEqual(m.artifact(json.dumps(row),meta,Path('/target')),Path('/target/debug/deps/test-binary'))
    def test_artifact_wrong_owner_extra_path_and_json_reject(self):
        meta=graph();row=artifact_row(meta)
        for key,bad in [('fresh',True),('fresh',None),('package_id','other'),('executable','/elsewhere/test'),('profile',{'test':False}),('target',{'kind':['bin'],'src_path':'/fixture/lib.rs'})]:
            changed=dict(row);changed[key]=bad
            with self.assertRaises(ValueError):m.artifact(json.dumps(changed),meta,Path('/target'))
        for log in [json.dumps(row)+'\n'+json.dumps(row),'warning\n'+json.dumps(row),'{}']:
            with self.assertRaises(ValueError):m.artifact(log,meta,Path('/target'))
    def test_prepare_requires_outer_controller_before_any_write(self):
        with patch.dict(os.environ,{},clear=True),patch.object(m.prep,'prepare') as called:
            with self.assertRaises(ValueError):m.prepare_command(Path('/host'),Path('/fixture'))
            called.assert_not_called()
    def test_no_nested_metadata_or_subprocess_in_pure_preparer(self):
        source=(m.ROOT/m.PREP/'prepare.py').read_text()
        for token in ['Popen(','check_output(','import subprocess','start_new_session']:self.assertNotIn(token,source)
    def test_metadata_has_own_strict_stdout_and_locked_followup(self):
        source=(HERE/'run.py').read_text()
        self.assertIn("'fixture-metadata'",source);self.assertIn("'locked-metadata'",source)
        self.assertIn('if json.loads(locked)!=metadata',source)
        self.assertEqual(source.count('separate_stdout=True'),3)
    def test_binary_only_after_one_compile_and_all_seals(self):
        source=(HERE/'run.py').read_text()
        a=source.index("raw=runner.run('native-compile'");b=source.index('compiled=base.binary_seal');c=source.index("runner.run('inventory'");d=source.index("runner.run('generation6'")
        self.assertLess(a,b);self.assertLess(b,c);self.assertLess(c,d)
        self.assertEqual(source.count("'native-compile',"),1)
        for text in ['seal(ROOT)!=original','seal(host)!=host_before','seal(fixture)!=fixture_before','outputs(target)!=generated','base.binary_seal(binary)!=compiled']:self.assertIn(text,source)
    def test_generated_output_seals_include_mode_and_file_set(self):
        with tempfile.TemporaryDirectory() as tmp:
            root=Path(tmp);out=root/'debug/build/one/out';out.mkdir(parents=True);(out/'x').write_text('x')
            first=m.outputs(root);(out/'extra').write_text('y');self.assertNotEqual(first,m.outputs(root))
            (out/'extra').unlink();(out/'x').chmod(0o700);self.assertNotEqual(first,m.outputs(root))
    def test_setup_is_guarded_and_final_false_requires_all_gates(self):
        source=(HERE/'run.py').read_text()
        self.assertIn('runner.guard(reserve=2*1024**3)',source)
        for text in ["runner.run('apt-update'","runner.run('apt-install'","runner.run('toolchain'","set(report['gates'])!=REQUIRED","report['passed']=report['error'] is None and report['verified']"]:self.assertIn(text,source)
    def test_scope_excludes_unrun_old_suite_and_game(self):
        source=(HERE/'run.py').read_text();doc=(HERE/'README.md').read_text()
        self.assertIn("'consumer_tests::generation_tests::'",source)
        self.assertNotIn("'horizon-mixins'",source);self.assertNotIn("'-p','horizon-client'",source)
        for text in ['only compiled and inventoried','no Mixins','not rerun','No cache/artifact']:self.assertIn(text,doc)

if __name__=='__main__':unittest.main()
