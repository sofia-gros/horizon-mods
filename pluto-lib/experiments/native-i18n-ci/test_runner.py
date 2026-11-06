import contextlib,importlib.util,io,json,os,tempfile,unittest
from pathlib import Path
from types import SimpleNamespace
from unittest import mock
HERE=Path(__file__).resolve().parent
spec=importlib.util.spec_from_file_location('i18n_ci',HERE/'run.py');m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m)
class Tests(unittest.TestCase):
 def test_all_inputs_are_exact(self):
  for p,h in m.read_inputs()['inputs_sha256'].items():self.assertEqual(m.sha((m.ROOT/p).read_bytes()),h,p)
 def test_native_groups_are_complete_and_source_derived(self):
  c=m.read_inputs();record=json.loads((m.ROOT/'pluto-lib/docs/item-display-language-inputs-20261106.json').read_text())
  self.assertEqual(c['result_groups'],record['result_groups']);self.assertEqual([g['expected'] for g in c['result_groups']],[8,6,8])
  names=[n for g in c['result_groups'] for n in g['names']];self.assertEqual(len(names),22);self.assertEqual(len(set(names)),22)
  for g,path in zip(c['result_groups'],['pluto-lib/tests/native/item_display_language.rs',c['author'],c['language']]):
   source=(m.ROOT/path).read_text();self.assertEqual(source.count('#[test]'),g['expected'])
   for n in g['names']:self.assertIn('fn '+n.rsplit('::',1)[1]+'(',source)
 def test_full_suite_rejects_ignored_filtered_missing_and_duplicates(self):
  good='test a ... ok\ntest b ... ok\ntest result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.0s\n'
  self.assertEqual(m.validate_suite(good,2,['a','b']),['a','b'])
  for bad in ['',good.replace('test b','test a'),good.replace(' ... ok',' ... ignored',1),good.replace('0 filtered','1 filtered'),good.replace('2 passed','0 passed')]:
   with self.assertRaises(ValueError):m.validate_suite(bad,2,['a','b'])
 def test_real_compile_cannot_be_fresh_or_nontest(self):
  m.actual_compile('Running `rustc --crate-name real lib.rs --test`','real',True)
  for bad in ['Fresh real','Running `rustc --crate-name real lib.rs`','Running `rustc --crate-name other lib.rs --test`']:
   with self.assertRaises(ValueError):m.actual_compile(bad,'real',True)
 def test_regular_source_inventory_rejects_links(self):
  with tempfile.TemporaryDirectory(prefix='i18n-ci-test-') as d:
   p=Path(d);(p/'a').write_text('a');(p/'b').symlink_to(p/'a')
   with self.assertRaises(ValueError):m.regular_seal(p)
 def test_generated_inputs_and_recompile_checked(self):
  with tempfile.TemporaryDirectory(prefix='i18n-ci-test-') as d:
   p=Path(d);f=p/'target/debug/build/x/out/v';f.parent.mkdir(parents=True);f.write_text('before')
   good='test a ... ok\ntest result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.0s\n'
   runner=SimpleNamespace(work=p,run=lambda *_:good);self.assertEqual(m.test_after_compile(runner,'a',[],1,['a']),['a'])
   runner.run=lambda *_:good+'Running `rustc --crate-name other lib.rs`\n'
   with self.assertRaises(ValueError):m.test_after_compile(runner,'a',[],1,['a'])
   def drift(*_):f.write_text('after');return good
   runner.run=drift
   with self.assertRaises(ValueError):m.test_after_compile(runner,'a',[],1,['a'])
 def test_other_mount_capacity_is_checked(self):
  for limited in ['/','/rustup']:
   def usage(p):return SimpleNamespace(free=m.FLOOR-1 if str(p)==limited else m.FLOOR+3*1024**3)
   with mock.patch.object(m,'existing_parent',side_effect=Path),mock.patch.object(m.shutil,'disk_usage',side_effect=usage):
    with self.assertRaises(RuntimeError):m.Runner(Path('/work'),{'CARGO_HOME':'/cargo','RUSTUP_HOME':'/rustup'}).guard()
 def test_absent_rustup_checks_existing_parent(self):
  with tempfile.TemporaryDirectory(prefix='i18n-ci-test-') as d:self.assertEqual(m.existing_parent(Path(d)/'new'/'rustup'),Path(d).resolve())
 def test_python_count_cannot_skip(self):
  good='test_a (Tests.test) ... ok\nRan 1 test in 0.0s\nOK\n';m.validate_python_checks(good,1)
  for bad in ['',good.replace(' ... ok',' ... skipped'),good.replace('Ran 1','Ran 0'),good.replace('OK','OK (skipped=1)')]:
   with self.assertRaises(ValueError):m.validate_python_checks(bad,1)
 def test_workflow_scope(self):
  s=(m.ROOT/'.github/workflows/native-i18n-wip.yml').read_text()
  for v in [m.read_inputs()['branch'],'contents: read','private == false','persist-credentials: false','ubuntu-24.04','cancel-in-progress: true']:self.assertIn(v,s)
  for v in ['workflow_dispatch','pull_request:','actions/cache','upload-artifact','self-hosted','secrets.','contents: write']:self.assertNotIn(v,s)
 def test_setup_group_and_final_checks_are_present(self):
  s=(HERE/'run.py').read_text()
  for v in ['FLOOR+2*1024**3','os.killpg(proc.pid, 0)',"Own process group remains:",'proc.wait(timeout=2)',"git(ROOT,'status','--porcelain','--untracked-files=all')","git(official,'status','--porcelain','--untracked-files=all')","regular_seal(fixture)!=fixture_seal"]:self.assertIn(v,s)
  self.assertLess(s.index("runner.run('runner-checks'"),s.index("runner.run('toolchain'"))
 def test_scope_has_no_host_install_or_locale_activation(self):
  c=m.read_inputs();s=(HERE/'run.py').read_text()
  self.assertEqual(c['official']['url'],'https://github.com/horizon-rust/horizon.git')
  for v in ['sofia-gros/horizon.git','horizon-mixins','--allow-dirty','lang::set(','--bin','horizon-client-render']:self.assertNotIn(v,s)
  self.assertIn('not installed client library',s)
 def test_failure_and_missing_gate_cannot_be_success(self):
  report={'error':None};m.report_error(report,ValueError('one'));m.report_error(report,RuntimeError('two'),'verification: ')
  self.assertIn('one; verification: RuntimeError: two',report['error'])
  for failure in ['exception','unverified']:
   with tempfile.TemporaryDirectory(prefix='i18n-ci-test-') as d:
    def execute(report):
     if failure=='exception':raise ValueError('expected failure')
    with mock.patch.object(m,'execute',side_effect=execute),mock.patch.dict(os.environ,{'GITHUB_STEP_SUMMARY':str(Path(d)/'summary'),'GITHUB_SHA':'test'}),contextlib.redirect_stdout(io.StringIO()) as out:
     with self.assertRaises(SystemExit):m.main()
    report=json.loads(out.getvalue().split('FINAL_RECEIPT ',1)[1]);self.assertFalse(report['passed'])
 def test_metadata_and_exact_official_identities_are_verified_before_compile(self):
  s=(HERE/'run.py').read_text();self.assertLess(s.index("runner.run('metadata-resolve'"),s.index("runner.run('actual-native-compile'"))
  self.assertLess(s.index("runner.run('verify-before'"),s.index("runner.run('actual-native-compile'"))
  self.assertIn("runner.run('verify-after'",s)
  p=(m.ROOT/'pluto-lib/tools/prepare_item_display_language_fixture.py').read_text()
  for v in ["if got - allowed - {local}","path.is_relative_to(host / 'crates')","Expected one actual official package"]:self.assertIn(v,p)
 def test_original_author_example_and_production_modules_are_used(self):
  c=m.read_inputs();record=json.loads((m.ROOT/'pluto-lib/docs/item-display-language-inputs-20261106.json').read_text())
  for role in ['language','author','example']:
   self.assertEqual(m.sha((m.ROOT/c[role]).read_bytes()),record['adapters'][role]['sha256'])
  fixture=(m.ROOT/'pluto-lib/tests/native/item_display_language.rs').read_text()
  for v in ['#[path = "language.rs"] pub mod pluto_language;','#[path = "language-text.rs"] pub mod pluto_language_text;','extern crate self as horizon_client;','#[path = "language_text_example.rs"] mod author_example;']:self.assertIn(v,fixture)
if __name__=='__main__':unittest.main()
