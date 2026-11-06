"""Pure parser/source regressions; no compiler, linker, rustup or CI invocation."""
import ast
import contextlib
import importlib.util
import io
import json
import os
from pathlib import Path
import tempfile
import unittest
from unittest import mock

HERE=Path(__file__).resolve().parent
spec=importlib.util.spec_from_file_location('bounds_controller',HERE/'run.py')
m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m)


def cfg():return json.loads((HERE/'inputs.json').read_text())
def with_config(c):
    @contextlib.contextmanager
    def changed():
        with tempfile.TemporaryDirectory() as tmp:
            p=Path(tmp);(p/'inputs.json').write_text(json.dumps(c))
            with mock.patch.object(m,'HERE',p):yield
    return changed()


def micro(short=False,clock=1,values=None):
    values=values if values is not None else [2_000_000]*7
    return '\n'.join('LOCAL_BOUNDS_MICRO case='+name+' iterations=4096 warmup=128 samples=7 '+
        'old_ns='+json.dumps(values)+' candidate_ns='+json.dumps(values)+
        ' observed_clock_ns='+str(clock)+' inconclusive='+str(short).lower()
        for name in ['front6','paired12','nonlocal','empty'])


class Tests(unittest.TestCase):
    def test_01_complete_exact_sources(self):
        self.assertEqual(m.config(),cfg())
        self.assertEqual(len(m.INPUT_PATHS),19)

    def test_02_missing_extra_source_rejected(self):
        for mode in ('missing','extra'):
            c=cfg()
            if mode=='missing':c['source_sha256'].pop(next(iter(c['source_sha256'])))
            else:c['source_sha256']['foreign']='0'*64
            with with_config(c),self.assertRaises(ValueError):m.config()

    def test_03_scope_and_source_drift_rejected(self):
        for field in ('repository','branch','base'):
            c=cfg();c[field]='foreign'
            with with_config(c),self.assertRaises(ValueError):m.config()
        c=cfg();c['source_sha256'][next(iter(c['source_sha256']))]='0'*64
        with with_config(c),self.assertRaises(ValueError):m.config()

    def test_04_exact_full_qualified_names_derived(self):
        for mode in ('wrong','missing','duplicate','order'):
            c=cfg()
            if mode=='wrong':c['test_names'][0]='other::test'
            elif mode=='missing':c['test_names'].pop()
            elif mode=='duplicate':c['test_names'][0]=c['test_names'][1]
            else:c['test_names'].reverse()
            with with_config(c),self.assertRaises(ValueError):m.config()

    def test_05_generated_std_and_host_hashes_required(self):
        for field in ('generated_sha256','host_generated_sha256'):
            c=cfg();c[field]='0'*64
            with with_config(c),self.assertRaises(ValueError):m.config()

    def test_06_public_exact_event_only(self):
        with tempfile.TemporaryDirectory() as tmp:
            p=Path(tmp)/'event';p.write_text(json.dumps({'repository':{'private':False,'full_name':m.REPO}}))
            e={'GITHUB_EVENT_PATH':str(p),'GITHUB_ACTIONS':'true','GITHUB_REPOSITORY':m.REPO,
               'GITHUB_EVENT_NAME':'push','GITHUB_REF':'refs/heads/'+m.BRANCH,'GITHUB_SHA':'f'*40}
            with mock.patch.dict(os.environ,e,clear=True):m.event('f'*40)
            for key,value in [('GITHUB_ACTIONS','false'),('GITHUB_REF','refs/heads/main'),('GITHUB_EVENT_NAME','pull_request'),('GITHUB_SHA','0'*40)]:
                with mock.patch.dict(os.environ,{**e,key:value},clear=True),self.assertRaises(ValueError):m.event('f'*40)
            p.write_text(json.dumps({'repository':{'private':True,'full_name':m.REPO}}))
            with mock.patch.dict(os.environ,e,clear=True),self.assertRaises(ValueError):m.event('f'*40)

    def test_07_terse_complete_inventory_no_zero_or_extra(self):
        names=cfg()['test_names'];good=''.join(n+': test\n' for n in sorted(names));m.inventory(good,names)
        for bad in ['',good[:-1],good+good.splitlines()[0]+'\n',good.replace(': test',': benchmark',1),good+'14 tests, 0 benchmarks\n','\n'.join(reversed(good.splitlines()))+'\n']:
            with self.assertRaises(ValueError):m.inventory(bad,names)

    def test_08_all_four_micro_cases(self):
        rows=m.timings(micro());self.assertEqual(len(rows),4)
        self.assertFalse(any(r['timing_inconclusive'] for r in rows))

    def test_09_malformed_bounds_and_shapes_reject(self):
        for bad in [micro().replace('samples=7','samples=0'),micro(values=[1]*6),
                    micro().replace('2000000','true',1),micro(values=[2**64]*7),
                    micro(clock=0),micro(clock=2**64)]:
            with self.assertRaises(ValueError):m.timings(bad)

    def test_10_missing_duplicate_and_reordered_micro_reject(self):
        lines=micro().splitlines()
        for bad in ['\n'.join(lines[:-1]),'\n'.join(lines+[lines[0]]),'\n'.join(reversed(lines)),micro().replace('case=empty','case=foreign')]:
            with self.assertRaises(ValueError):m.timings(bad)

    def test_11_short_sample_and_clock_saturation_not_success(self):
        self.assertTrue(all(r['timing_inconclusive'] for r in m.timings(micro(True,values=[0]*7))))
        self.assertTrue(all(r['timing_inconclusive'] for r in m.timings(micro(True,clock=2**64-1))))
        for bad in [micro(False,values=[1]*7),micro(True)]:
            with self.assertRaises(ValueError):m.timings(bad)

    def test_12_exact_actual_compiler_arguments(self):
        for opt in (0,3):
            argv=m.compile_argv('/tool/rustc','/work/fixture.rs','/work/binary',opt)
            self.assertEqual(argv[0],'/tool/rustc');self.assertEqual(argv[-3:],['/work/fixture.rs','-o','/work/binary'])
            self.assertEqual([argv[i+1] for i,x in enumerate(argv[:-1]) if x=='-C'],
                ['opt-level='+str(opt),'debug-assertions=on','overflow-checks=on','debuginfo=0','codegen-units=16','lto=off'])
            self.assertEqual(argv.count('--test'),1);self.assertNotIn('cargo',argv)

    def test_13_unsupported_profiles_rejected(self):
        for value in (True,False,'3',1,2,4,None):
            with self.assertRaises(ValueError):m.compile_argv('/c','/s','/b',value)

    def test_14_binary_seal_rejects_link_and_nonexecutable(self):
        with tempfile.TemporaryDirectory() as tmp:
            p=Path(tmp)/'binary';p.write_bytes(b'not executed');p.chmod(0o700)
            self.assertEqual(m.binary_seal(p)['sha256'],m.sha(b'not executed'))
            link=p.with_name('link');link.symlink_to(p)
            with self.assertRaises(ValueError):m.binary_seal(link)
            p.chmod(0o600)
            with self.assertRaises(ValueError):m.binary_seal(p)

    def test_15_ignored_and_untracked_checkout_rejected(self):
        with mock.patch.object(m.A,'git',return_value='!! .cargo/config.toml'),self.assertRaises(ValueError):m.C.assert_checkout()
        with tempfile.TemporaryDirectory() as tmp:
            root=Path(tmp);(root/'ignored').write_bytes(b'x');before=m.C.strict_seal(root)
            (root/'new').write_bytes(b'y')
            with self.assertRaises(ValueError):m.C.assert_strict(root,before)

    def test_16_fixed_in_memory_compositions_only(self):
        raw,receipt=m.B.compose(m.ROOT);host,h=m.H.compose(m.ROOT)
        self.assertEqual((len(raw),len(host)),(24080,25005));self.assertEqual(receipt['prepared_tests'],14)
        self.assertTrue(h['original_host_fully_reconstructed']);self.assertFalse(h['manifest_created'])
        self.assertFalse(any(h['execution'].values()))

    def test_17_unguarded_child_never_executes(self):
        with mock.patch.dict(os.environ,{},clear=True),mock.patch.object(m.os,'execv') as execute:
            with self.assertRaises(ValueError):m.child()
            execute.assert_not_called()

    def test_18_hard_limits_precede_only_explicit_exec(self):
        source=(HERE/'run.py').read_text();tree=ast.parse(source)
        child=next(n for n in tree.body if isinstance(n,ast.FunctionDef) and n.name=='child')
        calls=[n for n in ast.walk(child) if isinstance(n,ast.Call) and isinstance(n.func,ast.Attribute)]
        limits=sorted(n.lineno for n in calls if n.func.attr=='setrlimit')
        execute=[n.lineno for n in calls if n.func.attr=='execv']
        self.assertEqual(len(limits),3);self.assertEqual(len(execute),1);self.assertLess(max(limits),execute[0])
        self.assertEqual(m.AS_LIMIT,1024**3)
        self.assertIn('os.getsid(0)!=os.getpid()',source)
        self.assertIn('event(A.git(ROOT',source)

    def test_19_own_log_and_all_filesystem_disk_floor(self):
        with tempfile.TemporaryDirectory() as tmp:
            root=Path(tmp);report={'phases':[]};runner=m.Runner(root,{'CARGO_HOME':str(root)},report)
            with mock.patch.object(m.A,'write_filesystems',return_value=['/workspace','/rustup']),mock.patch.object(m.A.shutil,'disk_usage',side_effect=[mock.Mock(free=m.A.FLOOR+100),mock.Mock(free=m.A.FLOOR-1)]):
                with self.assertRaises(RuntimeError):runner.guard()
            (root/'x.log').write_bytes(b'1234')
            with mock.patch.object(m.A.Runner,'guard'),mock.patch.object(m,'LOG_LIMIT',3):
                with self.assertRaises(RuntimeError):runner.guard()

    def test_20_failure_or_missing_gates_emit_false_receipt(self):
        def false_success(r):r['verified']=True
        for action in [ValueError('source refusal'),false_success]:
            out=io.StringIO()
            with mock.patch.object(m.sys,'argv',['run.py']),mock.patch.object(m,'execute',side_effect=action),contextlib.redirect_stdout(out):
                with self.assertRaises(SystemExit) as e:m.main()
                self.assertEqual(e.exception.code,1)
            row=json.loads(out.getvalue().split('FINAL_RECEIPT ',1)[1]);self.assertFalse(row['passed']);self.assertTrue(row['error'])


if __name__=='__main__':unittest.main()
