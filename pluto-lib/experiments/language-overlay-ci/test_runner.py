import contextlib,importlib.util,io,json,os,tempfile,unittest
from pathlib import Path
from types import SimpleNamespace
from unittest import mock
HERE=Path(__file__).resolve().parent
spec=importlib.util.spec_from_file_location('language_overlay_ci',HERE/'run.py');m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m)
class Tests(unittest.TestCase):
 def test_all_inputs_are_exact(self):
  for p,h in m.read_inputs()['inputs_sha256'].items():self.assertEqual(m.sha((m.ROOT/p).read_bytes()),h,p)
 def test_native_groups_are_complete_and_source_derived(self):
  c=m.read_inputs();record=json.loads((m.ROOT/'pluto-lib/experiments/language-overlay/inputs.json').read_text())
  self.assertEqual(c['result_groups'],[{'expected':8,'names':record['expected_test_names'],'prefix':'tests::'}])
  names=c['result_groups'][0]['names'];self.assertEqual(len(names),8);self.assertEqual(len(set(names)),8)
  source=(m.ROOT/'pluto-lib/experiments/language-overlay/fixture.rs').read_text();self.assertEqual(source.count('#[test]'),8)
  for n in names:self.assertIn('fn '+n.rsplit('::',1)[1]+'(',source)
 def test_full_suite_rejects_ignored_filtered_missing_and_duplicates(self):
  good='test a ... ok\ntest b ... ok\ntest result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.0s\n'
  self.assertEqual(m.validate_suite(good,2,['a','b']),['a','b'])
  for bad in ['',good.replace('test b','test a'),good.replace(' ... ok',' ... ignored',1),good.replace('0 filtered','1 filtered'),good.replace('2 passed','0 passed')]:
   with self.assertRaises(ValueError):m.validate_suite(bad,2,['a','b'])
 def test_real_compile_cannot_be_fresh_or_nontest(self):
  with contextlib.redirect_stdout(io.StringIO()):m.actual_compile('Running `rustc --crate-name real lib.rs --test`','real',True)
  for bad in ['Fresh real','Running `rustc --crate-name real lib.rs`','Running `rustc --crate-name other lib.rs --test`']:
   with self.assertRaises(ValueError):m.actual_compile(bad,'real',True)
 def test_regular_source_inventory_rejects_links(self):
  with tempfile.TemporaryDirectory(prefix='language-overlay-ci-test-') as d:
   p=Path(d);(p/'a').write_text('a');(p/'b').symlink_to(p/'a')
   with self.assertRaises(ValueError):m.regular_seal(p)
 def test_generated_inputs_and_recompile_checked(self):
  with tempfile.TemporaryDirectory(prefix='language-overlay-ci-test-') as d:
   p=Path(d);f=p/'target/debug/build/x/out/v';f.parent.mkdir(parents=True);f.write_text('before')
   good='test a ... ok\ntest result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.0s\n'
   runner=SimpleNamespace(work=p,run=lambda *_:good)
   with contextlib.redirect_stdout(io.StringIO()):self.assertEqual(m.test_after_compile(runner,'a',[],1,['a']),['a'])
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
  with tempfile.TemporaryDirectory(prefix='language-overlay-ci-test-') as d:self.assertEqual(m.existing_parent(Path(d)/'new'/'rustup'),Path(d).resolve())
 def test_python_count_cannot_skip(self):
  good='test_a (Tests.test) ... ok\nRan 1 test in 0.0s\nOK\n';m.validate_python_checks(good,1)
  for bad in ['',good.replace(' ... ok',' ... skipped'),good.replace('Ran 1','Ran 0'),good.replace('OK','OK (skipped=1)')]:
   with self.assertRaises(ValueError):m.validate_python_checks(bad,1)
 def test_workflow_scope(self):
  s=(m.ROOT/'.github/workflows/language-overlay-wip.yml').read_text()
  for v in [m.read_inputs()['branch'],'contents: read','private == false','persist-credentials: false','ubuntu-24.04','cancel-in-progress: true']:self.assertIn(v,s)
  for v in ['workflow_dispatch','pull_request:','actions/cache','upload-artifact','self-hosted','secrets.','contents: write']:self.assertNotIn(v,s)
 def test_setup_group_and_final_checks_are_present(self):
  s=(HERE/'run.py').read_text()
  for v in ['FLOOR+2*1024**3','os.killpg(proc.pid, 0)',"Own process group remains:",'proc.wait(timeout=2)',"git(ROOT,'status','--porcelain','--untracked-files=all','--ignored=matching')","git(official,'status','--porcelain','--untracked-files=all','--ignored=matching')","regular_seal(fixture)!=fixture_seal"]:self.assertIn(v,s)
  self.assertLess(s.index("runner.run('runner-checks'"),s.index("runner.run('toolchain'"))
 def test_scope_has_no_host_install_or_locale_activation(self):
  c=m.read_inputs();s=(HERE/'run.py').read_text()
  self.assertEqual(c['official']['url'],'https://github.com/horizon-rust/horizon.git')
  m.validate_official(c)
  self.assertLess(s.index('validate_official(config);'),s.index("runner.run('toolchain'"))
  self.assertLess(s.index('validate_official(config);'),s.index("runner.run('official-fetch'"))
  for field in ['url','revision','lock_sha256']:
   changed=m.read_inputs();changed['official'][field]='unapproved'
   with self.assertRaises(ValueError):m.validate_official(changed)
  for v in ['sofia-gros/horizon.git','horizon-mixins','--allow-dirty','lang::set(','--bin','--test-threads=2']:self.assertNotIn(v,s)
  self.assertIn('No installed Game admission',s)
 def test_failure_and_missing_gate_cannot_be_success(self):
  report={'error':None};m.report_error(report,ValueError('one'));m.report_error(report,RuntimeError('two'),'verification: ')
  self.assertIn('one; verification: RuntimeError: two',report['error'])
  for failure in ['exception','unverified','missing_gates']:
   with tempfile.TemporaryDirectory(prefix='language-overlay-ci-test-') as d:
    def execute(report):
     if failure=='exception':raise ValueError('expected failure')
     if failure=='missing_gates':report['verified']=True
    with mock.patch.object(m,'execute',side_effect=execute),mock.patch.dict(os.environ,{'GITHUB_STEP_SUMMARY':str(Path(d)/'summary'),'GITHUB_SHA':'test'}),contextlib.redirect_stdout(io.StringIO()) as out:
     with self.assertRaises(SystemExit):m.main()
    report=json.loads(out.getvalue().split('FINAL_RECEIPT ',1)[1]);self.assertFalse(report['passed'])
 def test_metadata_and_exact_official_identities_are_verified_before_compile(self):
  s=(HERE/'run.py').read_text();self.assertLess(s.index("runner.run('metadata-resolve'"),s.index("runner.run('actual-native-compile'"))
  self.assertLess(s.index("runner.run('verify-before'"),s.index("runner.run('actual-native-compile'"))
  self.assertIn("runner.run('verify-after'",s)
  p=(HERE/'prepare.py').read_text()
  for v in ["identity(p) not in allowed|{local}","path.is_relative_to(host/'crates')","Expected one actual official package"]:self.assertIn(v,p)
 def test_actual_package_and_language_owners_are_required(self):
  s=(HERE/'run.py').read_text()
  self.assertIn("['horizon_client_core','horizon_mods','horizon_client_lib']",s)
  self.assertIn("actual_compile(built,'pluto_language_overlay_fixture',True)",s)
  for v in ["'native22'","horizon_items","horizon_world","source = \"registry"]:self.assertNotIn(v,s)
  fixture=(m.ROOT/'pluto-lib/experiments/language-overlay/fixture.rs').read_text()
  for v in ['Package::write_files(', 'Package::read(', 'Content::read(', 'mod_content::build(', 'Language::load(&[&out]']:self.assertIn(v,fixture)
 def test_setup_source_gate_uses_real_official_root_before_fetch(self):
  s=(HERE/'run.py').read_text()
  self.assertLess(s.index("runner.run('official-checkout'"),s.index("runner.run('source-checks'"))
  self.assertLess(s.index("runner.run('source-checks'"),s.index("runner.run('official-locked-fetch'"))
  self.assertIn("m.HOST=Path(sys.argv[2])",s)
 def test_preparer_metadata_rejects_other_local_owner(self):
  spec=importlib.util.spec_from_file_location('package_prepare',HERE/'prepare.py');p=importlib.util.module_from_spec(spec);spec.loader.exec_module(p)
  with tempfile.TemporaryDirectory(prefix='language-overlay-ci-test-') as d:
   root=Path(d).resolve();host=root/'official';out=root/'fixture'
   local=lambda name,path:{'name':name,'source':None,'manifest_path':str(path/'Cargo.toml')}
   good={'packages':[local(p.NAME,out),local('horizon-mods',host/'crates/horizon-mods'),local('horizon-client-core',host/'crates/horizon-client-core'),local('horizon-client-lib',host/'crates/horizon-client-lib')]}
   p.verify_metadata(good,out,host)
   for bad in [good['packages']+[good['packages'][0]],good['packages'][:-1],good['packages']+[local('intruder',root/'elsewhere')]]:
    with self.assertRaises(ValueError):p.verify_metadata({'packages':bad},out,host)
 def test_no_runtime_mutation_or_overlay_claim(self):
  c=m.read_inputs();self.assertEqual(c['cutoffs']['fixture'],'01ccfa4f115f06aed312931f1057adc958aac5de')
  s=(HERE/'run.py').read_text()
  for v in ['No installed Game admission','selected/server pack merge or filters','runtime TOML claim']:self.assertIn(v,s)
  p=(HERE/'prepare.py').read_text()
  self.assertIn("if out.exists():raise FileExistsError(out)",p)
  self.assertNotIn('remove_dir',p)
 def test_lock_rejects_unknown_package_or_unresolved_fixture(self):
  spec=importlib.util.spec_from_file_location('package_prepare_lock',HERE/'prepare.py');p=importlib.util.module_from_spec(spec);spec.loader.exec_module(p)
  with tempfile.TemporaryDirectory(prefix='language-overlay-ci-test-') as d:
   root=Path(d);host=root/'official';out=root/'fixture';host.mkdir();out.mkdir()
   package=lambda name:'[[package]]\nname = "'+name+'"\nversion = "0.0.0"\n'
   base=package('horizon-mods')+package('horizon-client-core')+package('horizon-client-lib')
   (host/'Cargo.lock').write_text(base)
   good=base+package(p.NAME);(out/'Cargo.lock').write_text(good);p.verify_lock(out,host)
   for bad in [base,good+package('unexpected'),good+package(p.NAME)]:
    (out/'Cargo.lock').write_text(bad)
    with self.assertRaises(ValueError):p.verify_lock(out,host)
 def test_checkout_seal_tracks_ignored_new_files_and_link_targets(self):
  with tempfile.TemporaryDirectory(prefix='language-overlay-ci-test-') as d:
   root=Path(d);(root/'tracked').write_text('before');(root/'link').symlink_to('tracked')
   original=m.seal(root,True);m.assert_seal(root,original,True)
   extra=root/'.cargo';extra.mkdir();(extra/'config.toml').write_text('new')
   with self.assertRaises(ValueError):m.assert_seal(root,original,True)
   (extra/'config.toml').unlink();extra.rmdir();(root/'link').unlink();(root/'link').symlink_to('other')
   with self.assertRaises(ValueError):m.assert_seal(root,original,True)
class OverlayCI(unittest.TestCase):
 def test_fixed_branch_base_rejected_before_setup(self):
  for field in ['repository','branch','base']:
   c=m.read_inputs();c[field]='unexpected'
   with self.assertRaises(ValueError):m.validate_official(c)
 def test_bounded_console_does_not_truncate_validation_input(self):
  with tempfile.TemporaryDirectory(prefix='language-overlay-ci-test-') as d:
   p=Path(d)/'raw.log';raw=b'noise'*50000+b'END';p.write_bytes(raw)
   with contextlib.redirect_stdout(io.StringIO()) as out:m.emit_log_summary('compile',p,0)
   text=out.getvalue();self.assertLess(len(text),500);self.assertIn(m.sha(raw),text)
   self.assertEqual(p.read_bytes(),raw)
   with contextlib.redirect_stdout(io.StringIO()) as out:m.emit_log_summary('compile',p,1)
   self.assertLess(len(out.getvalue()),17000);self.assertIn('END',out.getvalue())
 def test_validated_names_only_follow_strict_raw_validation(self):
  good='test a ... ok\ntest result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.0s\n'
  with tempfile.TemporaryDirectory(prefix='language-overlay-ci-test-') as d:
   runner=SimpleNamespace(work=Path(d),run=lambda *_:good)
   with contextlib.redirect_stdout(io.StringIO()) as out:m.test_after_compile(runner,'case',[],1,['a'])
   self.assertIn('VALIDATED_TEST_NAMES',out.getvalue())
   runner.run=lambda *_:good.replace('... ok','... ignored')
   with contextlib.redirect_stdout(io.StringIO()) as out:
    with self.assertRaises(ValueError):m.test_after_compile(runner,'case',[],1,['a'])
   self.assertNotIn('VALIDATED_TEST_NAMES',out.getvalue())
 def test_source_only_fixture_scope_remains_distinct(self):
  c=m.read_inputs();self.assertEqual(c['python_tests'],6)
  self.assertEqual(c['cutoffs']['successful_package_runner'],'8b17fba9610c3e60a873c822f2fa04f93d5c94e4')
  s=(HERE/'run.py').read_text()
  for v in ['selected/server pack merge or filters','priority repair','Options/live UI']:
   self.assertIn(v,s)
  for v in ['horizon-mixins','apt-install','--no-default-features']:
   self.assertNotIn(v,s)
if __name__=='__main__':unittest.main()
