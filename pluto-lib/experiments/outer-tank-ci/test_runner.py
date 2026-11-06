import importlib.util
import json
import contextlib
import io
import os
import ast
from pathlib import Path
import tempfile
from types import SimpleNamespace
import unittest
from unittest import mock

HERE=Path(__file__).resolve().parent
def module(name,path):
    spec=importlib.util.spec_from_file_location(name,path);value=importlib.util.module_from_spec(spec);spec.loader.exec_module(value);return value
m=module('outer_ci',HERE/'run.py')
gate=module('outer_rustc_gate',HERE/'rustc_gate.py')

class Tests(unittest.TestCase):
    def test_frozen_configuration_and_complete_api(self):
        c=m.read_inputs()
        self.assertEqual(c['source_revision'],'794a0c526ca0a96d0ca9c9ff40010611b5968a89')
        self.assertEqual(c['official']['revision'],'624c4f6265c02296fbe8a80f029dd0d987fb94c0')
        self.assertEqual(len(c['files']),55);self.assertEqual(len(c['api_files']),28)
        self.assertEqual(set(c['api_files']),{p for p in c['files'] if p.startswith(c['api_path']+'/')})
        self.assertEqual((c['tests'],c['new_tests'],c['inherited_tests']),(160,4,156))
        self.assertEqual(len(set(c['required_tests'])),4)
        for p,row in c['files'].items():
            self.assertFalse(Path(p).is_absolute());self.assertNotIn('..',Path(p).parts)
            self.assertRegex(row['sha256'],r'^[a-f0-9]{64}$');self.assertGreater(row['bytes'],0)

    def test_preparation_is_inverse_bound_and_stops_before_cargo(self):
        c=m.read_inputs();original=(m.ROOT/c['fixture_runner']).read_text()
        actual=m.prepare_prefix(original,c['files'][c['fixture_runner']]['sha256'])
        restored=actual.replace("root = Path(__import__('sys').argv[2]).resolve()",'root = Path(__file__).resolve().parents[2]')
        restored=restored.replace("with __import__('contextlib').nullcontext(__import__('sys').argv[1]) as temp:","with tempfile.TemporaryDirectory(prefix='outer-tank-patch-') as temp:")
        self.assertEqual(restored,original.split('    lock_fixture_to_host(manifest, host, root)\n')[0])
        self.assertNotIn('subprocess.run(',actual)
        for text in ['production +','scaffold = scaffold[:scaffold.index',"oracle.replace('fn tank_to_nbt_preserving('"]:
            self.assertIn(text,actual)

    def test_preparation_rejects_drift_or_nonunique_boundary(self):
        c=m.read_inputs();s=(m.ROOT/c['fixture_runner']).read_text();h=c['files'][c['fixture_runner']]['sha256']
        with self.assertRaises(ValueError):m.prepare_prefix(s+'\n',h)
        for changed in [s.replace('root = Path(__file__).resolve().parents[2]','root = Path("other")'),s+'    lock_fixture_to_host(manifest, host, root)\n']:
            with self.assertRaises(ValueError):m.prepare_prefix(changed,m.sha(changed.encode()))

    def test_suite_demands_every_test_without_skip_or_filter(self):
        names=['case_'+str(n) for n in range(160)]
        good=''.join('test '+n+' ... ok\n' for n in names)+'test result: ok. 160 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.1s\n'
        self.assertEqual(len(m.validate_suite(good,160,names[:4])),160)
        for bad in ['',good.replace('test case_1 ...','test case_0 ...'),good.replace(' ... ok',' ... ignored',1),good.replace('0 filtered','1 filtered'),good.replace('160 passed','159 passed')]:
            with self.assertRaises(ValueError):m.validate_suite(bad,160,names[:4])
        with self.assertRaises(ValueError):m.validate_suite(good,160,['absent_boundary'])

    def test_actual_compiler_evidence_not_fresh(self):
        m.actual_compile('Running `rustc --crate-name outer_tank_patch_fixture lib.rs --test`','outer_tank_patch_fixture',True)
        for bad in ['Fresh outer_tank_patch_fixture','Running `rustc --crate-name outer_tank_patch_fixture lib.rs`']:
            with self.assertRaises(ValueError):m.actual_compile(bad,'outer_tank_patch_fixture',True)

    def test_sealed_compiler_gate_denies_before_execution(self):
        compile=['--crate-name','outer_tank_patch_fixture','lib.rs','--test','--emit=dep-info,link']
        self.assertEqual(gate.decision('compile',compile),'compile')
        self.assertEqual(gate.decision('sealed',compile),'denied')
        self.assertEqual(gate.decision('invalid',['-vV']),'denied')
        for query in [['-vV'],['--version'],['-','--crate-name','___','--print=file-names','--crate-type','bin','--print=sysroot','-Wwarnings']]:
            self.assertEqual(gate.decision('sealed',query),'query')
        for bad in [['file.rs','--print=cfg'],['--print=sysroot','--emit=link'],['-','--crate-name','real','--print=cfg']]:
            self.assertEqual(gate.decision('sealed',bad),'denied')

    def test_native_metadata_rejects_substitute_or_duplicate(self):
        paths=m.package_paths(Path('/official'),Path('/source'),Path('/fixture'))
        packages=[{'name':n,'source':None,'manifest_path':str(p/'Cargo.toml')} for n,p in paths.items()]
        m.assert_metadata({'packages':packages},paths)
        for changed in [packages[:-1],packages+[packages[0]],[{**packages[0],'manifest_path':'/replacement/Cargo.toml'},*packages[1:]]]:
            with self.assertRaises(ValueError):m.assert_metadata({'packages':changed},paths)
        self.assertEqual(len(paths),6)

    def test_identity_lock_allows_only_original_and_two_packages(self):
        with tempfile.TemporaryDirectory() as tmp:
            host=Path(tmp)/'host';fixture=Path(tmp)/'fixture';host.mkdir();fixture.mkdir()
            seed='version=4\n[[package]]\nname="a"\nversion="1"\nsource="registry+https://example.test"\nchecksum="abc"\n'
            (host/'Cargo.lock').write_text(seed);(fixture/'Cargo.lock').write_text(seed+'[[package]]\nname="pluto-inventory-api"\nversion="0.1.0"\n')
            m.verify_lock(host,fixture)
            (fixture/'Cargo.lock').write_text(seed.replace('checksum="abc"','checksum="drift"'))
            with self.assertRaises(ValueError):m.verify_lock(host,fixture)

    def test_seal_rejects_added_files_and_symlinks(self):
        with tempfile.TemporaryDirectory() as tmp:
            root=Path(tmp);(root/'x').write_text('x');before=m.regular_seal(root)
            (root/'other').write_text('y')
            with self.assertRaises(ValueError):m.assert_seal(root,before)
            (root/'link').symlink_to(root/'x')
            with self.assertRaises(ValueError):m.regular_seal(root)

    def test_disk_guard_checks_other_output_filesystems(self):
        for low in ['/','/rustup']:
            usage=lambda p:SimpleNamespace(free=m.FLOOR-1 if str(p)==low else m.FLOOR+6*1024**3)
            with mock.patch.object(m,'existing_parent',side_effect=Path),mock.patch.object(m.shutil,'disk_usage',side_effect=usage),mock.patch.object(m,'check_memory',return_value=8*1024**3):
                with self.assertRaises(RuntimeError):m.Runner(Path('/work'),{'CARGO_HOME':'/cargo','RUSTUP_HOME':'/rustup'}).guard()

    def test_memory_unknown_or_low_fails_closed(self):
        with mock.patch.object(Path,'read_text',return_value='MemTotal: 8388608 kB\nMemAvailable: 4194304 kB\n'):
            with self.assertRaises(ValueError):m.check_memory(1)
        with mock.patch.object(m,'check_memory',side_effect=RuntimeError('RAM limit')),mock.patch.object(m.shutil,'disk_usage',return_value=SimpleNamespace(free=m.FLOOR+8*1024**3)):
            with self.assertRaises(RuntimeError):m.Runner(Path('/tmp'),{'CARGO_HOME':'/tmp'}).guard()

    def test_execution_rejects_compiler_log_or_out_dir_changes(self):
        with tempfile.TemporaryDirectory() as tmp:
            work=Path(tmp);out=work/'target/debug/build/example/out';out.mkdir(parents=True);file=out/'value';file.write_text('before')
            good='test a ... ok\ntest result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.0s\n'
            runner=SimpleNamespace(work=work,run=lambda *a,**kw:good)
            self.assertEqual(m.test_after_compile(runner,'test',[],1,['a']),['a'])
            runner.run=lambda *a,**kw:good+'Running `rustc --crate-name unexpected lib.rs`\n'
            with self.assertRaises(ValueError):m.test_after_compile(runner,'test',[],1,['a'])
            def change(*a,**kw):file.write_text('after');return good
            runner.run=change
            with self.assertRaises(ValueError):m.test_after_compile(runner,'test',[],1,['a'])

    def test_workflow_is_dedicated_readonly_and_no_credentials_or_cache(self):
        s=(m.ROOT/'.github/workflows/outer-tank-wip.yml').read_text()
        for wanted in [m.read_inputs()['branch'],'contents: read','private == false','persist-credentials: false','runs-on: ubuntu-24.04','cancel-in-progress: true']:
            self.assertIn(wanted,s)
        for absent in ['workflow_dispatch','pull_request:','actions/cache','upload-artifact','secrets.','self-hosted','contents: write']:
            self.assertNotIn(absent,s)

    def test_final_source_checks_and_process_cleanup_are_mandatory(self):
        s=(HERE/'run.py').read_text()
        for wanted in ['os.killpg(proc.pid, 0)','Own process group remains','Dirty/untracked checkout after proof','Dirty/untracked official source','minimum_available_memory_bytes','runner.guard(reserve=4*1024**3)','assert_seal(host,official_before,True)']:
            self.assertIn(wanted,s)
        self.assertNotIn('/workspace/scratch',s)
        self.assertIn('mock Game/world',m.read_inputs()['scope'])

    def test_initial_or_summary_failure_cannot_emit_success(self):
        with tempfile.TemporaryDirectory() as tmp:
            for kind in ['initial','unverified','summary']:
                def execute(report):
                    if kind=='initial':raise ValueError('initial failure')
                    if kind=='summary':report['verified']=True
                summary=Path(tmp)/('absent/file' if kind=='summary' else 'summary')
                with mock.patch.object(m,'execute',side_effect=execute),mock.patch.dict(os.environ,{'GITHUB_STEP_SUMMARY':str(summary)}),contextlib.redirect_stdout(io.StringIO()) as stdout:
                    with self.assertRaises(SystemExit):m.main()
                receipt=json.loads(stdout.getvalue().split('FINAL_RECEIPT ',1)[1]);self.assertFalse(receipt['passed'])

    def test_missing_memory_facts_are_observations_not_unlimited(self):
        with tempfile.TemporaryDirectory() as tmp:
            root=Path(tmp);proc=root/'proc';group=root/'cgroup';(proc/'self').mkdir(parents=True);group.mkdir()
            (proc/'meminfo').write_text('MemTotal: 8388608 kB\nMemAvailable: 4194304 kB\n')
            (proc/'self/cgroup').write_text('0::/\n')
            (proc/'self/mountinfo').write_text('1 0 0:1 / /sys/fs/cgroup rw - cgroup2 cgroup rw\n')
            got=m.memory_observation(proc,group)
            self.assertEqual(got['host_memory']['MemAvailable'],'4194304 kB')
            self.assertEqual(len(got['cgroup2_mounts']),1)
            self.assertIn('error',got['visible_ancestors'][0]['facts']['memory.max'])
            self.assertIn('not determined',got['admission'])
            self.assertNotIn('unlimited',json.dumps(got))

    def test_diagnostic_mode_exits_before_setup_or_native_path(self):
        report={}
        with mock.patch.object(m,'memory_observation',return_value={'unverified':True}),mock.patch.object(m,'Runner',side_effect=AssertionError('setup must not start')),mock.patch.object(m.subprocess,'Popen',side_effect=AssertionError('native must not start')),contextlib.redirect_stdout(io.StringIO()):
            with self.assertRaisesRegex(RuntimeError,'cannot start setup or compilation'):
                m.diagnostic_only(report,m.read_inputs())
        self.assertFalse(report['setup_started']);self.assertFalse(report['native_started'])
        tree=ast.parse((HERE/'run.py').read_text())
        calls=[n.func.id for n in ast.walk(tree) if isinstance(n,ast.Call) and isinstance(n.func,ast.Name)]
        self.assertNotIn('execute_native_prepared',calls)
        with self.assertRaises(ValueError):m.diagnostic_only({}, {'execution_mode':'native'})

if __name__=='__main__':unittest.main()
