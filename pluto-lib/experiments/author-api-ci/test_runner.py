import importlib.util,json,tempfile,unittest,os
from pathlib import Path
from unittest import mock
from types import SimpleNamespace
HERE=Path(__file__).resolve().parent
spec=importlib.util.spec_from_file_location('author_ci',HERE/'run.py');m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m)
class Tests(unittest.TestCase):
 def test_full_suites_have_exact_source_sets(self):
  c=m.read_inputs();root=HERE/'source' if (HERE/'source').is_dir() else m.ROOT
  for key,count,files in [('inventory',147,33),('public',58,11)]:
   s=c['apis'][key];self.assertEqual(s['tests'],count);self.assertEqual(len(s['files']),files)
   self.assertEqual(m.regular_seal(root/s['path']),s['files'])
   actual=sum((root/s['path']/p).read_text().count('#[test]') for p in s['files'] if p.startswith('src/') and p.endswith('.rs'))
   self.assertEqual(actual,count)
  self.assertEqual(len(c['apis']['inventory']['required_tests']),13)
  self.assertEqual(len(c['apis']['public']['required_tests']),6)
 def test_all_input_hashes(self):
  root=HERE/'source' if (HERE/'source').is_dir() else m.ROOT
  for p,h in m.read_inputs()['inputs_sha256'].items():self.assertEqual(m.sha((root/p).read_bytes()),h,p)
 def test_suite_rejects_skips_filters_duplicates(self):
  good='test a ... ok\ntest b ... ok\ntest result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.0s\n'
  self.assertEqual(m.validate_suite(good,2,['a']),['a','b'])
  for bad in ['',good.replace('test b','test a'),good.replace(' ... ok',' ... ignored',1),good.replace('0 filtered','1 filtered'),good.replace('2 passed','0 passed')]:
   with self.assertRaises(ValueError):m.validate_suite(bad,2,['a'])
 def test_actual_compile_cannot_be_fresh_or_nontest(self):
  m.actual_compile('Running `rustc --crate-name real src/lib.rs --test`','real',True)
  for bad in ['Fresh real','Running `rustc --crate-name real src/lib.rs`','Running `rustc --crate-name replacement src/lib.rs --test`']:
   with self.assertRaises(ValueError):m.actual_compile(bad,'real',True)
 def test_metadata_rejects_duplicate_or_substitute_local_package(self):
  expected={'a':Path('/tmp/a'),'b':Path('/tmp/b')}
  packages=[{'name':n,'source':None,'manifest_path':str(p/'Cargo.toml')} for n,p in expected.items()]
  m.assert_metadata({'packages':packages},expected)
  for wrong in [packages[:1],packages+[packages[0]],[packages[0],{**packages[1],'manifest_path':'/tmp/fake/Cargo.toml'}]]:
   with self.assertRaises(ValueError):m.assert_metadata({'packages':wrong},expected)
 def test_no_symlinks_in_real_api(self):
  with tempfile.TemporaryDirectory(prefix='author-api-check-') as d:
   r=Path(d);(r/'file').write_text('x');(r/'link').symlink_to(r/'file')
   with self.assertRaises(ValueError):m.regular_seal(r)
 def test_guard_other_output_filesystem(self):
  for limited in ['/','/rustup']:
   def usage(p):return SimpleNamespace(free=m.FLOOR-1 if str(p)==limited else m.FLOOR+3*1024**3)
   with mock.patch.object(m,'existing_parent',side_effect=Path),mock.patch.object(m.shutil,'disk_usage',side_effect=usage):
    with self.assertRaises(RuntimeError):m.Runner(Path('/work'),{'CARGO_HOME':'/cargo','RUSTUP_HOME':'/rustup'}).guard()
 def test_nonexistent_rustup_is_checked_at_parent(self):
  with tempfile.TemporaryDirectory(prefix='author-api-check-') as d:self.assertEqual(m.existing_parent(Path(d)/'new'/'rustup'),Path(d).resolve())
 def test_workflow_scope(self):
  path=HERE/'workflow.yml' if (HERE/'workflow.yml').exists() else m.ROOT/'.github/workflows/author-api-wip.yml'
  w=path.read_text()
  for s in [m.read_inputs()['branch'],'contents: read','private == false','persist-credentials: false','runs-on: ubuntu-24.04','cancel-in-progress: true']:self.assertIn(s,w)
  for s in ['workflow_dispatch','pull_request:','actions/cache','upload-artifact','secrets.','self-hosted','contents: write']:self.assertNotIn(s,w)
 def test_all_setup_is_guarded_and_untracked_final_is_rejected(self):
  s=(HERE/'run.py').read_text()
  self.assertIn('FLOOR+2*1024**3',s);self.assertIn("runner.run('toolchain'",s)
  self.assertIn("git(ROOT,'status','--porcelain','--untracked-files=all')",s)
  self.assertIn("raise ValueError('Dirty/untracked checkout after proof')",s)
  self.assertIn('os.killpg(proc.pid, 0)',s);self.assertIn("raise RuntimeError('Own process group remains: '",s)
 def test_test_execution_rejects_recompile_and_generated_drift(self):
  with tempfile.TemporaryDirectory(prefix='author-api-check-') as d:
   work=Path(d);out=work/'target/debug/build/example/out';out.mkdir(parents=True);f=out/'value';f.write_text('before')
   good='test a ... ok\ntest result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.0s\n'
   runner=SimpleNamespace(work=work,run=lambda *_:good)
   self.assertEqual(m.test_after_compile(runner,'test',[],1,['a']),['a'])
   runner.run=lambda *_:good+'Running `rustc --crate-name other src/lib.rs`\n'
   with self.assertRaises(ValueError):m.test_after_compile(runner,'test',[],1,['a'])
   def change(*_):f.write_text('changed');return good
   runner.run=change
   with self.assertRaises(ValueError):m.test_after_compile(runner,'test',[],1,['a'])
 def test_real_seed_dependencies_are_shared(self):
  root=HERE/'source' if (HERE/'source').is_dir() else m.ROOT;c=m.read_inputs()
  inv=m.identities(root/c['apis']['inventory']['lock']);pub=m.identities(root/c['apis']['public']['lock'])
  registry=lambda rows:{r for r in rows if r[2] is not None}
  self.assertEqual(registry(inv),registry(pub));self.assertEqual(len(registry(inv)),11)
 def test_scope_excludes_host_and_keeps_storage_assertions(self):
  c=m.read_inputs();root=HERE/'source' if (HERE/'source').is_dir() else m.ROOT
  self.assertEqual(c['storage_bins'],['storage','storage_public'])
  self.assertEqual(len(c['consumer_bins']),4)
  for key in c['storage_bins']:
   storage=(root/c['consumer_bins'][key]).read_text()
   self.assertEqual(storage.count('assert_eq!')+storage.count('assert!'),6);self.assertIn('planned.expect(',storage)
  legacy=(root/c['consumer_bins']['storage']).read_text();public=(root/c['consumer_bins']['storage_public']).read_text()
  self.assertEqual(public,legacy.replace('use pluto_inventory_api::{plan_between, storage::stage_updates, MoveError, StackSnapshot};','use pluto_inventory_api::inventory::{plan_between, stage_updates, MoveError, StackSnapshot};'))
  s=(HERE/'run.py').read_text()
  for bad in ['horizon-mixins','horizon-client','--allow-dirty','sofia-gros/horizon.git']:self.assertNotIn(bad,s)
if __name__=='__main__':unittest.main()
