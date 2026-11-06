import importlib.util
from pathlib import Path
import tempfile
import re
import unittest
from unittest import mock
from types import SimpleNamespace
HERE=Path(__file__).resolve().parent
spec=importlib.util.spec_from_file_location('fluid_ci',HERE/'run.py');m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m)
class SourceChecks(unittest.TestCase):
    def test_exact_test_groups_have_fixed_counts(self):
        config=m.read_inputs()
        for field,count in [('std_tests',30),('fluid_tests',21),('native_tests',2)]:
            names=config[field];self.assertEqual(len(names),count);self.assertEqual(len(set(names)),count)
            log='\n'.join('test '+n+' ... ok' for n in names)+f'\ntest result: ok. {count} passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.0s\n'
            m.validate_tests(log,names,0)
    def test_std_qualified_names_match_exact_source_modules(self):
        config=m.read_inputs();actual=[]
        native=m.ROOT/'pluto-lib/mixins/pluto_native_creative'
        for outer,filename in config['std_modules'].items():
            source=(native/filename).read_text()
            # The frozen std files have top-level cfg(test) modules and ordinary
            # one-level test functions. Fail closed if this source shape changes.
            for found in re.finditer(r'^mod (\w+) \{\n(.*?)^\}',source,re.M|re.S):
                for name in re.findall(r'^    #\[test\]\n    fn (\w+)\(',found[2],re.M):
                    actual.append(outer+'::'+found[1]+'::'+name)
        self.assertEqual(len(actual),30)
        self.assertEqual(set(actual),set(config['std_tests']))
        self.assertIn('pluto_terrain_bindings::revision_view_tests::stale_before_inspection_is_explicitly_unavailable',actual)
        self.assertIn('pluto_terrain_bindings::appearance_tests::legacy_empty_and_single_appearance_validate',actual)
    def test_all_source_inventories_and_filters_are_exact(self):
        config=m.read_inputs();m.validate_test_inventory(config)
        import copy
        bad=copy.deepcopy(config);bad['fluid_filter']='fluid::tests::'
        with self.assertRaises(ValueError):m.validate_test_inventory(bad)
        bad=copy.deepcopy(config)
        bad['fluid_tests']=[n.replace('::revision_tests::','::tests::') for n in bad['fluid_tests']]
        with self.assertRaises(ValueError):m.validate_test_inventory(bad)
        bad=copy.deepcopy(config);bad['native_filter']='unknown::'
        with self.assertRaises(ValueError):m.validate_test_inventory(bad)
        self.assertLess((HERE/'run.py').read_text().index('    validate_test_inventory(config)'),(HERE/'run.py').read_text().index("runner.run('apt-update'"))
    def test_missing_skipped_duplicate_or_zero_fails(self):
        names=['example::one','example::two']
        good='test example::one ... ok\ntest example::two ... ok\ntest result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 1 filtered out; finished in 0.1s\n'
        for log in ['',good.replace(' ... ok',' ... ignored',1),good.replace('2 passed','0 passed'),good+'test example::one ... ok\n',good.replace('example::two','example::unknown')]:
            with self.assertRaises(ValueError):m.validate_tests(log,names)
    def test_actual_compiler_invocations_required(self):
        with self.assertRaises(ValueError):m.native_rustc('Fresh horizon_client_render',['horizon_client_render'])
        m.native_rustc('Running `rustc --crate-name horizon_client_render --edition=2024`',['horizon_client_render'])
    def test_host_library_binary_and_client_lib_are_distinct(self):
        common='\n'.join('Running `rustc --crate-name '+n+' src/lib.rs`' for n in ['pluto_api','horizon_client_render','horizon_client_lib'])
        lib='Running `rustc --crate-name horizon_client src/lib.rs`'
        binary='Running `rustc --crate-name horizon_client src/main.rs`'
        m.native_host_checks(common+'\n'+lib+'\n'+binary)
        for log in [common+'\n'+binary, common+'\n'+lib, ('Fresh horizon_client_lib\n'+lib+'\n'+binary), (common+'\n'+lib+'\n'+binary).replace('--crate-name horizon_client_lib ','--crate-name substitute_lib ')]:
            with self.assertRaises(ValueError):m.native_host_checks(log)
    def test_python_guard_count_cannot_be_zero_or_skipped(self):
        log='\n'.join('test_'+str(i)+' (Checks.test) ... ok' for i in range(19))+'\nRan 19 tests in 0.001s\n\nOK\n'
        m.validate_source_checks(log)
        for bad in ['Ran 0 tests in 0.001s\nOK\n',log.replace(' ... ok',' ... skipped',1),log.replace('Ran 19','Ran 18'),log.replace('\nOK\n','\nOK (skipped=1)\n')]:
            with self.assertRaises(ValueError):m.validate_source_checks(bad)
    def test_separate_output_mounts_and_absent_rustup_parent(self):
        with tempfile.TemporaryDirectory(prefix='fluid-ci-guard-') as root:self.assertEqual(m.existing_parent(Path(root)/'missing'/'rustup'),Path(root).resolve())
        for limited in ['/','/rustup-alt']:
            def usage(path):return SimpleNamespace(free=m.FLOOR-1 if str(path)==limited else m.FLOOR+3*1024**3)
            with mock.patch.object(m,'existing_parent',side_effect=Path),mock.patch.object(m.shutil,'disk_usage',side_effect=usage):
                with self.assertRaises(RuntimeError):m.Runner(Path('/work'),{'CARGO_HOME':'/cargo','RUSTUP_HOME':'/rustup-alt'}).guard()
    def test_workflow_is_exact_public_push_no_cache_artifact(self):
        w=(m.ROOT/'.github/workflows/fluid-render-wip.yml').read_text()
        for token in ['- '+m.read_inputs()['branch'],'contents: read','github.event.repository.private == false','persist-credentials: false','runs-on: ubuntu-24.04','cancel-in-progress: true']:
            self.assertIn(token,w)
        for token in ['workflow_dispatch','pull_request:','actions/cache','upload-artifact','secrets.','self-hosted','contents: write']:self.assertNotIn(token,w)
    def test_guards_precede_apply_and_all_outputs_are_sealed(self):
        s=(HERE/'run.py').read_text()
        self.assertLess(s.index("runner.run('all12guards'"),s.index("runner.run('official-apply'"))
        for token in ['assert_seal(stage,stage_before)','assert_seal(ROOT,before_mod,True)',"out_dirs(work/'host-target')!=generated","config['bundle_sha256']","runner.guard(reserve=2*1024**3)",'proc.wait(timeout=2)']:
            self.assertIn(token,s)
    def test_old_cpu_fixture_or_foreign_source_not_used(self):
        s=(HERE/'run.py').read_text()
        for token in ['terrain-cell-cpu','real_native_mesh_differential','--allow-dirty','sofia-gros/horizon.git']:
            self.assertNotIn(token,s)
        self.assertEqual(m.read_inputs()['official']['horizon']['url'],'https://github.com/horizon-rust/horizon.git')
        self.assertEqual(m.read_inputs()['official']['mixins']['url'],'https://github.com/horizon-rust/horizon-mixins.git')
    def test_package_unpack_rejects_traversal_and_link(self):
        import zipfile
        with tempfile.TemporaryDirectory(prefix='fluid-ci-zip-') as root:
            root=Path(root)
            for i,(name,mode) in enumerate([('../escape',0o100644),('link',0o120777)]):
                p=root/f'{i}.zip'
                with zipfile.ZipFile(p,'w') as z:
                    info=zipfile.ZipInfo(name);info.external_attr=mode<<16;z.writestr(info,b'x')
                with self.assertRaises(ValueError):m.unpack(p,root/f'out{i}')
if __name__=='__main__':unittest.main()
