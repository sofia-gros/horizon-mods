import importlib.util
from pathlib import Path
import unittest
from unittest import mock
import tempfile
from types import SimpleNamespace
HERE=Path(__file__).resolve().parent
spec=importlib.util.spec_from_file_location('guide_ci',HERE/'run.py')
m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m)
class Checks(unittest.TestCase):
    def setUp(self):
        self.c=m.read_inputs()
        self.log='\n'.join('test '+n+' ... ok' for n in self.c['test_names'])+'\ntest result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 11 filtered out; finished in 0.01s\n'
    def test_exact_names_and_count(self):m.validate_tests(self.log,self.c)
    def test_zero_or_skipped_is_not_success(self):
        for text in ['',self.log.replace('12 passed','0 passed'),self.log.replace(' ... ok',' ... ignored',1)]:
            with self.assertRaises(ValueError):m.validate_tests(text,self.c)
    def test_duplicate_unknown_missing_rejected(self):
        for text in [self.log+'test '+self.c['test_names'][0]+' ... ok\n',self.log.replace(self.c['test_names'][0],'consumer_tests::other'),self.log.split('\n',1)[1]]:
            with self.assertRaises(ValueError):m.validate_tests(text,self.c)
    def test_actual_three_compilers_required(self):
        log='\n'.join('Running `rustc --crate-name '+n+' --edition=2024`' for n in ['horizon_client_core','horizon_client_render','pluto_guide_text_tests'])
        m.native_rustc(log)
        with self.assertRaises(ValueError):m.native_rustc(log.replace('horizon_client_render','modeled_render'))
    def test_offline_and_post_seal_gates_are_explicit(self):
        source=(HERE/'run.py').read_text()
        for token in ["'official-locked-fetch'","'--locked'","'--offline'","'verify-after-compile'","'verify-after-tests'","out_dirs(target) != generated","start_new_session=True","signal.SIGTERM","signal.SIGKILL"]:
            self.assertIn(token,source)
        self.assertLess(source.index("'official-locked-fetch'"),source.index("runner.run('prepare'"))
    def test_setup_is_inside_continuous_guard(self):
        source=(HERE/'run.py').read_text()
        for phase in ['apt-update','apt-install','toolchain-install']:
            self.assertIn("runner.run('"+phase+"'",source)
        workflow=(HERE.parents[2]/'.github/workflows/guide-font-wip.yml').read_text()
        self.assertNotIn('sudo apt-get',workflow)
        self.assertNotIn('rustup toolchain install',workflow)
    def test_setup_headroom_and_child_shutdown_are_bounded(self):
        source=(HERE/'run.py').read_text()
        self.assertIn('runner.guard(reserve=2 * 1024**3)',source)
        self.assertIn('time.monotonic() + 1.0',source)
        self.assertIn('proc.wait(timeout=2)',source)
        self.assertNotIn('proc.wait(timeout=10)',source)
        self.assertIn("env['CARGO_HOME'], Path('/')",source)
        self.assertIn("env.get('RUSTUP_HOME', str(Path.home()/'.rustup'))",source)
        self.assertIn('existing_parent(p)',source)
    def test_missing_rustup_path_uses_existing_parent(self):
        with tempfile.TemporaryDirectory(prefix='guide-ci-guard-') as root:
            self.assertEqual(m.existing_parent(Path(root)/'missing'/'rustup'),Path(root).resolve())
    def test_root_and_rustup_separate_mounts_can_stop_guard(self):
        for limited in ['/','/rustup-separate']:
            seen=[]
            def usage(path):
                seen.append(str(path))
                return SimpleNamespace(free=m.FLOOR-1 if str(path)==limited else m.FLOOR+3*1024**3)
            with mock.patch.object(m,'ROOT',Path('/checkout')), mock.patch.object(m,'existing_parent',side_effect=Path), mock.patch.object(m.shutil,'disk_usage',side_effect=usage):
                runner=m.Runner(Path('/work'),{'CARGO_HOME':'/cargo','RUSTUP_HOME':'/rustup-separate'})
                with self.assertRaises(RuntimeError):runner.guard()
            self.assertEqual(set(seen),{'/checkout','/work','/cargo','/','/rustup-separate'})
    def test_workflow_is_only_exact_branch_readonly_no_cache_artifacts(self):
        workflow=(HERE.parents[2]/'.github/workflows/guide-font-wip.yml').read_text()
        self.assertIn('push:\n    branches:\n      - '+self.c['branch'],workflow)
        for token in ['github.event.repository.private == false','contents: read','persist-credentials: false','runs-on: ubuntu-24.04','timeout-minutes: 55']:
            self.assertIn(token,workflow)
        for token in ['workflow_dispatch','pull_request:','contents: write','actions/cache','upload-artifact','secrets.','self-hosted']:
            self.assertNotIn(token,workflow)
if __name__=='__main__':unittest.main()
