import contextlib
import copy
import importlib.util
import io
import json
from pathlib import Path
import tempfile
from types import SimpleNamespace
import unittest
from unittest import mock

HERE = Path(__file__).resolve().parent
spec=importlib.util.spec_from_file_location('storage_ci',HERE/'run.py')
m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m)

class Checks(unittest.TestCase):
    def test_exact_prototype_inputs_and_eleven_delta_files(self):
        c=m.config();self.assertEqual(len(c['source_sha256']),9)
        self.assertEqual(len(c['expected_delta']),11)
        self.assertEqual(c['prototype_publication'],'b434e9a4c11f355875ed0f3df38a2474d2f20478')
        self.assertFalse(any(Path(p).name in {'Cargo.toml','Cargo.lock'} for p in c['expected_delta']))
        self.assertIn('crates/horizon-client/src/game/retained_book.rs',c['expected_delta'])

    def test_only_authorized_public_push(self):
        env={'GITHUB_ACTIONS':'true','GITHUB_REPOSITORY':m.REPOSITORY,'GITHUB_EVENT_NAME':'push',
             'GITHUB_REF':'refs/heads/'+m.BRANCH,'GITHUB_SHA':'abc'}
        event={'repository':{'private':False,'full_name':m.REPOSITORY}}
        m.validate_event(env,event,'abc')
        for key in env:
            bad=dict(env);bad[key]='wrong'
            with self.assertRaises(ValueError):m.validate_event(bad,event,'abc')
        with self.assertRaises(ValueError):m.validate_event(env,{'repository':{'private':True}},'abc')

    def sample_metadata(self, root):
        root=str(root)
        p={'name':'horizon-client','id':'path+file://'+root+'/crates/horizon-client#0.1.0','source':None,
           'manifest_path':root+'/crates/horizon-client/Cargo.toml','targets':[
               {'name':'horizon_client','kind':['lib'],'src_path':root+'/crates/horizon-client/src/lib.rs'},
               {'name':'horizon-client','kind':['bin'],'src_path':root+'/crates/horizon-client/src/main.rs'}]}
        return {'workspace_root':root,'packages':[p],'workspace_members':[p['id']],
                'resolve':{'nodes':[{'id':p['id'],'deps':[],'features':[]}]}}

    def test_metadata_compares_full_graph_under_root_relocation(self):
        a=self.sample_metadata('/tmp/one');b=self.sample_metadata('/tmp/two')
        self.assertEqual(m.metadata(json.dumps(a),Path('/tmp/one')),m.metadata(json.dumps(b),Path('/tmp/two')))
        b['resolve']['nodes'][0]['features']=['unexpected']
        self.assertNotEqual(m.metadata(json.dumps(a),Path('/tmp/one')),m.metadata(json.dumps(b),Path('/tmp/two')))

    def test_metadata_rejects_external_local_paths_or_incomplete_graph(self):
        for case in ['manifest','target','resolve','member']:
            value=self.sample_metadata('/tmp/one')
            if case=='manifest':value['packages'][0]['manifest_path']='/tmp/other/Cargo.toml'
            elif case=='target':value['packages'][0]['targets'][0]['src_path']='/tmp/other/lib.rs'
            elif case=='resolve':value['resolve']=None
            else:value['workspace_members'].append('outside')
            with self.assertRaises(ValueError):m.metadata(json.dumps(value),Path('/tmp/one'))

    def test_metadata_requires_real_gameplay_lib_and_bin(self):
        for removed in [0,1]:
            value=self.sample_metadata('/tmp/one');value['packages'][0]['targets'].pop(removed)
            with self.assertRaises(ValueError):m.metadata(json.dumps(value),Path('/tmp/one'))

    def test_metadata_stdout_is_separate_from_stderr(self):
        proc=mock.Mock(pid=123456789,returncode=0)
        proc.poll.return_value=0;proc.wait.return_value=0
        def launch(*args,**kwargs):
            kwargs['stdout'].write('{"valid":true}\n');kwargs['stdout'].flush()
            kwargs['stderr'].write('Cargo warning on stderr\n');kwargs['stderr'].flush()
            return proc
        with tempfile.TemporaryDirectory() as t, mock.patch.object(m.subprocess,'Popen',side_effect=launch), mock.patch.object(m.os,'killpg',side_effect=ProcessLookupError), contextlib.redirect_stdout(io.StringIO()):
            runner=m.Runner(Path(t),{})
            with mock.patch.object(runner,'guard'):
                result=runner.run('metadata',['unused'],separate_stdout=True)
            self.assertEqual(json.loads(result),{'valid':True})
            self.assertEqual((Path(t)/'metadata.stderr.log').read_text(),'Cargo warning on stderr\n')
        text=(HERE/'run.py').read_text()
        self.assertEqual(text.count('cwd=pristine,separate_stdout=True'),1)
        self.assertEqual(text.count('cwd=experiment,separate_stdout=True'),1)

    def test_actual_compiler_invocations_not_fresh_or_mock(self):
        good='\n'.join(['Running `rustc --crate-name horizon_client_lib ../horizon-client/src/client_lib.rs --emit=metadata`',
                        'Running `rustc --crate-name horizon_client src/lib.rs --emit=metadata`',
                        'Running `rustc --crate-name horizon_client src/main.rs --emit=metadata`'])
        self.assertEqual(len(m.host_invocations(good)),3)
        for bad in ['Fresh horizon-client',good.replace('src/main.rs','mock.rs'),good+'\n'+good.splitlines()[0]]:
            with self.assertRaises(ValueError):m.host_invocations(bad)

    def test_delta_rejects_extra_or_missing_files(self):
        with tempfile.TemporaryDirectory() as t:
            root=Path(t);a=root/'a';b=root/'b';a.mkdir();b.mkdir()
            for p in [a,b]:(p/'Cargo.lock').write_text('locked');(p/'same.rs').write_text('original')
            (b/'new.rs').write_text('new'); expected={'new.rs':m.sha(b'new')}
            m.verify_delta(a,b,expected)
            (b/'extra.rs').write_text('oops')
            with self.assertRaises(ValueError):m.verify_delta(a,b,expected)
            (b/'extra.rs').unlink();(b/'same.rs').unlink()
            with self.assertRaises(ValueError):m.verify_delta(a,b,expected)

    def test_delta_rejects_wrong_hash_and_changed_manifest(self):
        with tempfile.TemporaryDirectory() as t:
            root=Path(t);a=root/'a';b=root/'b';a.mkdir();b.mkdir()
            for p in [a,b]:(p/'Cargo.lock').write_text('locked');(p/'Cargo.toml').write_text('manifest')
            (b/'new.rs').write_text('new')
            with self.assertRaises(ValueError):m.verify_delta(a,b,{'new.rs':'0'*64})
            (b/'Cargo.toml').write_text('altered')
            with self.assertRaises(ValueError):m.verify_delta(a,b,{'new.rs':m.sha(b'new')})

    def test_python_result_parser_rejects_empty_skipped_duplicate(self):
        good=''.join(f'test_{i} (Checks) ... ok\n' for i in range(7))+'Ran 7 tests in 0.1s\nOK\n'
        m.python_pass(good,7)
        for bad in ['',good+good,good.replace(' ... ok',' ... skipped',1),good.replace('7 tests','6 tests')]:
            with self.assertRaises(ValueError):m.python_pass(bad,7)

    def test_all_write_filesystems_and_reserve_are_guarded(self):
        for low in ['/', '/rustup', '/cargo', '/work', str(m.ROOT)]:
            with mock.patch.object(m.base,'existing',side_effect=Path),mock.patch.object(m.base.shutil,'disk_usage',side_effect=lambda p:SimpleNamespace(free=m.FLOOR-1 if str(p)==low else m.FLOOR+3*1024**3)):
                with self.assertRaises(RuntimeError):m.Runner(Path('/work'),{'RUSTUP_HOME':'/rustup','CARGO_HOME':'/cargo'}).guard()
        self.assertIn('runner.guard(reserve=2*1024**3)',(HERE/'run.py').read_text())

    def test_no_success_receipt_on_failure_missing_gate_or_summary_error(self):
        def incomplete(r):r['verified']=True
        def complete(r):r.update(verified=True,gates={k:True for k in m.REQUIRED})
        for execute,output in [(RuntimeError('compile'),mock.mock_open()),(incomplete,mock.mock_open()),(complete,mock.Mock(side_effect=OSError('summary')))]:
            with mock.patch.object(m,'execute',side_effect=execute),mock.patch('builtins.open',output),mock.patch.dict(m.os.environ,{'GITHUB_STEP_SUMMARY':'/summary'}),contextlib.redirect_stdout(io.StringIO()) as stream:
                with self.assertRaises(SystemExit):m.main()
            self.assertFalse(json.loads(stream.getvalue().split('FINAL_RECEIPT ')[1])['passed'])

    def test_compact_logs_and_bounded_process_group_teardown(self):
        source=(HERE/'run.py').read_text()
        self.assertIn('raw[-65536:]',source);self.assertNotIn('print(raw.decode()',source)
        self.assertIn('LOG_LIMIT = 128*1024**2',source)
        self.assertIn('proc.wait(timeout=2); base.ensure_group_gone(proc.pid, name)',source)
        with mock.patch.object(m.base.os,'killpg'),mock.patch.object(m.base.time,'monotonic',side_effect=[0,2]):
            with self.assertRaises(RuntimeError):m.base.ensure_group_gone(12,'remaining')

    def test_experiment_cannot_write_the_pristine_tree(self):
        m.independent_roots(Path('/tmp/source'),Path('/tmp/destination'))
        for a,b in [('/tmp/a','/tmp/a'),('/tmp/a','/tmp/a/child'),('/tmp/a/child','/tmp/a')]:
            with self.assertRaises(ValueError):m.independent_roots(Path(a),Path(b))
        source=(HERE/'run.py').read_text()
        self.assertIn('shutil.copytree(pristine, experiment)',source)
        self.assertIn('target = experiment/name',source)
        self.assertIn("if seal(pristine) != before",source)
        self.assertNotIn("target = pristine/name",source)
        self.assertIn("seals[str(experiment)]=verify_delta",source)
        self.assertIn("if seal(Path(name))!=before",source)

    def test_workflow_has_no_install_or_unreviewed_trigger(self):
        source=(m.ROOT/'.github/workflows/guide-storage-type-wip.yml').read_text()
        for value in ['contents: read','private == false','persist-credentials: false',m.BRANCH,'ubuntu-24.04']:
            self.assertIn(value,source)
        for value in ['workflow_dispatch','pull_request:','actions/cache','upload-artifact','secrets.','self-hosted','contents: write']:
            self.assertNotIn(value,source)
        runner=(HERE/'run.py').read_text()
        self.assertNotIn("'horizon-mixins'",runner)
        self.assertNotIn('package_native.py',runner)
        self.assertNotIn("'sudo'",runner)
        self.assertNotIn("'apt-get'",runner)

if __name__=='__main__':unittest.main()
