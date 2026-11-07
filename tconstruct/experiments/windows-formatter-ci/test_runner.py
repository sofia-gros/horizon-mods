"""Pure/mock only. No downloads, Windows API, formatter or subprocess is run."""
import ctypes
import ast
import io
import os
from types import SimpleNamespace
import hashlib
import importlib.util
import json
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import Mock,patch
HERE=Path(__file__).resolve().parent
sys.path.insert(0,str(HERE))
import run as r
import winjob

class Tests(unittest.TestCase):
    def test_shared_phase_is_exact_successful_source(self):
        self.assertEqual(r.base.sha((HERE.parent/'windows-source-ci/run.py').read_bytes()),'1d6f6a9b0122033cb7a4dbeeaa1e24cd9ad2385a762eb3c2d75b1ab3083ea3fa')
        self.assertEqual(r.base.TIMEOUT,120)
        self.assertEqual(r.base.LOG_LIMIT,128*1024)
    def test_three_process_job_exact_inverse(self):
        s=(HERE/'winjob.py').read_text(encoding='utf-8')
        s=s.replace('Own three-process formatter Job Object only.','Own two-process Job Object only.').replace('info.basic.active=3 # barrier worker + guard Python + one sequential rustfmt','info.basic.active=2 # barrier worker + one exact parser/test process').replace('# Header + at most three IDs;','# Header + at most two IDs;').replace('8+3*C.sizeof(C.c_size_t)','8+2*C.sizeof(C.c_size_t)')
        self.assertEqual(hashlib.sha256(s.encode()).hexdigest(),'2c3ff616596dde7caae785d12acacc12fdd0d3096538b1b51877066147cd738d')
    def test_layout_and_three_process_limit(self):
        self.assertEqual(ctypes.sizeof(winjob.Basic),64);self.assertEqual(ctypes.sizeof(winjob.Limits),144)
        s=(HERE/'winjob.py').read_text(encoding='utf-8')
        self.assertIn('info.basic.active=3',s);self.assertIn('8+3*C.sizeof',s)
    def test_assignment_precedes_worker_release(self):
        s=(HERE.parent/'windows-source-ci/run.py').read_text(encoding='utf-8')
        self.assertLess(s.index('job.assign(process)'),s.index("process.stdin.write(b'G')"))
        self.assertLess(s.index("sys.stdin.buffer.read(1)"),s.index('return subprocess.run(sys.argv[2:]'))
    def test_low_space_never_starts(self):
        with tempfile.TemporaryDirectory() as d,patch.object(r.base,'disk_guard',side_effect=RuntimeError('floor')),patch.object(r.base.subprocess,'Popen') as child,patch.object(winjob,'Job') as job:
            with self.assertRaisesRegex(RuntimeError,'floor'):r.base.phase('x',['known'],Path(d),{}, {'phases':[]})
            child.assert_not_called();job.assert_not_called()
    def test_assignment_failure_kills_barrier_only(self):
        process=Mock();job=Mock();job.assign.side_effect=RuntimeError('denied')
        with tempfile.TemporaryDirectory() as d,patch.object(r.base,'disk_guard'),patch.object(winjob,'Job',return_value=job),patch.object(r.base.subprocess,'Popen',return_value=process),patch.object(r.base.subprocess,'CREATE_NEW_PROCESS_GROUP',0,create=True):
            with self.assertRaisesRegex(RuntimeError,'denied'):r.base.phase('x',['known'],Path(d),{}, {'phases':[]})
        process.stdin.write.assert_not_called();process.kill.assert_called_once();job.terminate.assert_not_called();job.close.assert_called_once()
    def test_success_terminates_and_checks_own_job(self):
        process=Mock();process.poll.return_value=0;process.returncode=0;job=Mock();receipt={'phases':[]}
        with tempfile.TemporaryDirectory() as d,patch.object(r.base,'disk_guard'),patch.object(winjob,'Job',return_value=job),patch.object(r.base.subprocess,'Popen',return_value=process),patch.object(r.base.subprocess,'CREATE_NEW_PROCESS_GROUP',0,create=True):
            self.assertEqual(r.base.phase('x',['known'],Path(d),{},receipt),b'')
        job.terminate.assert_called_once();job.wait_empty.assert_called_once();job.close.assert_called_once();self.assertTrue(receipt['phases'][0]['job_empty'])
    def test_timeout_terminates_same_job(self):
        process=Mock();process.poll.return_value=None;job=Mock();receipt={'phases':[]}
        with tempfile.TemporaryDirectory() as d,patch.object(r.base,'disk_guard'),patch.object(winjob,'Job',return_value=job),patch.object(r.base.subprocess,'Popen',return_value=process),patch.object(r.base.subprocess,'CREATE_NEW_PROCESS_GROUP',0,create=True),patch.object(r.base.time,'monotonic',side_effect=[0,121,122]):
            with self.assertRaisesRegex(RuntimeError,'timeout'):r.base.phase('x',['known'],Path(d),{},receipt)
        job.terminate.assert_called_once();job.wait_empty.assert_called_once();job.close.assert_called_once();self.assertFalse(receipt['phases'][0]['process_succeeded'])
    def test_log_limit_before_read(self):
        with tempfile.TemporaryDirectory() as d:
            p=Path(d)/'x';p.write_bytes(b'x'*(r.base.LOG_LIMIT+1))
            with self.assertRaisesRegex(RuntimeError,'log limit'):r.base.read_log(p)
    def test_unknown_download_never_opens(self):
        with patch.object(r.probe,'source_map',return_value={}),patch.object(r.probe.urllib.request,'build_opener') as network:
            with self.assertRaisesRegex(ValueError,'Unlisted'):r.probe.download('https://unknown.invalid/x',Path('unused'),'0'*64,10)
            network.assert_not_called()
    def test_actual_invocation_is_one_fixed_formatter(self):
        exe=Path('C:/fixed with space/rustfmt.exe');args=[str(exe),'--edition','2024','--emit','stdout'];line=r.probe.subprocess.list2cmdline(args)
        for name in (None,str(exe)):self.assertEqual(r.probe.audit_call(name,line,exe)['argv'],args)
        for program,argv in [('rustfmt',line),(str(exe),line+' --config other'),(None,args),(None,'cargo build')]:
            with self.assertRaises(ValueError):r.probe.audit_call(program,argv,exe)
    def test_plan_is_exact_historical_bytes(self):
        with tempfile.TemporaryDirectory() as d:
            p=Path(d)/'plan';p.write_bytes(b'{}')
            with self.assertRaisesRegex(ValueError,'Historical'):r.probe.verify_plan(p)
    def test_formatter_version_is_not_inferred(self):
        r.probe.fmt.validate_version('rustfmt 1.10.0-stable (58cf8f9c35 2026-10-01)')
        for bad in ['rustfmt 1.9.0-stable (58cf8f9c35 2026-10-01)','rustfmt 1.10.0-stable (abcdef1234 2026-10-01)','rustfmt 1.10.0']:
            with self.assertRaises(ValueError):r.probe.fmt.validate_version(bad)
    def test_archive_mismatch_before_tar(self):
        with patch.object(r.probe.fmt.tarfile,'open') as tar:
            with self.assertRaises(ValueError):r.probe.fmt.archive_binary(b'not pinned')
            tar.assert_not_called()
    def test_exact_four_official_sources(self):
        self.assertEqual(r.probe.source_map(),r.config()['official_source_sha256'])
        self.assertEqual(len(r.probe.source_map()),4)
        self.assertTrue(r.probe.BASE.startswith('https://raw.githubusercontent.com/horizon-rust/horizon/41f822'))
    def test_expected_sequential_invocation_counts(self):
        self.assertEqual(r.probe.COUNTS,{'damage':3,'terrain-cell':4})
        s=(HERE/'probe.py').read_text(encoding='utf-8')
        self.assertIn("sys.addaudithook(audit)",s);self.assertIn("len(calls)>=COUNTS[kind]",s)
        self.assertIn('guard.verify_source(plan,official,guard.entries())',s);self.assertIn('guard.verify_regeneration(official,guard.metadata())',s)
        run=(HERE/'run.py').read_text(encoding='utf-8')
        self.assertLess(run.index("child_status.phase(base,'stat-pure'"),run.index("phase_json('prepare'"))
        self.assertIn('stat-pure',r.REQUIRED)
        self.assertEqual(len(r.config()['stat_test_names']),12)
    def test_runtime_and_guard_sources_remain_pinned(self):
        c=r.config()
        for p,h in c['dependency_sha256'].items():self.assertEqual(r.base.sha((r.ROOT/p).read_bytes()),h,p)
        self.assertEqual(c['runtime_base'],'aebe7a5d5e778efe273ae1afc2236ff8c3046274')
    def test_strict_named_count_and_no_skips(self):
        names=['test_a'];raw=b'test_a (__main__.Tests.test_a) ... ok\n\n'+b'-'*70+b'\nRan 1 tests in 0.010s\n\nOK\n'
        self.assertEqual(r.validation.suite(raw,names)['passed'],1)
        for bad in [b'',raw+raw,raw.replace(b'... ok',b'... skipped'),b'warning\n'+raw]:
            with self.assertRaises(ValueError):r.validation.suite(bad,names)
    def test_no_cargo_apply_or_game_entry(self):
        s=(HERE/'probe.py').read_text(encoding='utf-8');rtext=(HERE/'run.py').read_text(encoding='utf-8')
        self.assertNotIn('cargo build',s+rtext);self.assertNotIn('stage.py',s+rtext);self.assertNotIn('play_windows_latest',s+rtext)
        self.assertIn("'latest_composition_verified':False",rtext);self.assertIn("'game_build_executed':False",rtext)
    def test_preflight_rejects_nonwindows_without_network(self):
        with patch.object(r.os,'name','posix'),patch.object(r.probe.urllib.request,'build_opener') as network:
            with self.assertRaisesRegex(ValueError,'Windows'):r.preflight({})
            network.assert_not_called()
    def test_controller_delta_inverts_to_exact_eb19(self):
        source=(HERE/'run.py').read_text(encoding='utf-8')
        self.assertEqual(source.count('import child_status\n'),1)
        source=source.replace('import child_status\n','',1).replace('child_status.phase(base,','base.phase(')
        self.assertEqual(hashlib.sha256(source.encode()).hexdigest(),'8ffb0a369a9204d5bee6ed8ea1e636b0ec3ef40b3e8b3d7a00e33d0570aa9121')
    def record(self,argv,code=0):return {'schema':1,'argv_sha256':r.child_status.argv_hash(argv),'kind':'exit','returncode':code}
    def test_same_worker_retains_raw_unsigned_windows_code(self):
        c=r.child_status;argv=['fixed-rustfmt.exe','--version']
        with tempfile.TemporaryDirectory() as d:
            path=Path(d)/'version.child-status.json';env={c.STATUS_ENV:str(path),'PATH':'fixed'}
            with patch.object(c.sys,'argv',['worker','--worker',*argv]),patch.object(c.sys,'stdin',SimpleNamespace(buffer=io.BytesIO(b'G'))),patch.object(c.Path,'cwd',return_value=Path(d)),patch.dict(c.os.environ,env,clear=True),patch.object(c.subprocess,'run',return_value=SimpleNamespace(returncode=0xc0000135)) as child:
                self.assertEqual(c.worker(),0xc0000135)
                child.assert_called_once_with(argv,env={'PATH':'fixed'},check=False)
            self.assertEqual(c.read_status(path,argv)['result']['returncode'],0xc0000135)
    def test_barrier_rejection_never_starts_child_or_status(self):
        c=r.child_status
        with patch.object(c.sys,'stdin',SimpleNamespace(buffer=io.BytesIO(b''))),patch.object(c.subprocess,'run') as child,patch.object(c,'write_status') as write:
            self.assertEqual(c.worker(),97);child.assert_not_called();write.assert_not_called()
    def test_spawn_error_is_separate_from_exit_status(self):
        c=r.child_status;argv=['fixed'];failure=OSError(2,'not found')
        with tempfile.TemporaryDirectory() as d:
            path=Path(d)/'x.child-status.json'
            with patch.object(c.sys,'argv',['worker','--worker',*argv]),patch.object(c.sys,'stdin',SimpleNamespace(buffer=io.BytesIO(b'G'))),patch.object(c.Path,'cwd',return_value=Path(d)),patch.dict(c.os.environ,{c.STATUS_ENV:str(path)},clear=True),patch.object(c.subprocess,'run',side_effect=failure):
                with self.assertRaises(OSError) as caught:c.worker()
                self.assertIs(caught.exception,failure)
            value=c.read_status(path,argv)['result'];self.assertEqual(value['kind'],'spawn_error');self.assertEqual(value['errno'],2);self.assertNotIn('returncode',value)
    def test_status_path_is_fresh_fixed_and_same_work(self):
        c=r.child_status
        with tempfile.TemporaryDirectory() as d,patch.object(c.Path,'cwd',return_value=Path(d)):
            root=Path(d);good=root/'phase.child-status.json';self.assertEqual(c.status_path(str(good)),good)
            for bad in ['relative',str(root.parent/'phase.child-status.json'),str(root/'other.json')]:
                with self.assertRaises(ValueError):c.status_path(bad)
            good.write_bytes(b'{}')
            with self.assertRaises(ValueError):c.status_path(str(good))
            with self.assertRaises(FileExistsError):c.write_status(good,self.record(['a']))
    def test_status_parser_exact_identity_fields_and_numeric_bounds(self):
        c=r.child_status;argv=['a'];valid=self.record(argv)
        with tempfile.TemporaryDirectory() as d:
            p=Path(d)/'x.child-status.json'
            for code in (0,-1,-2**31,2**32-1):
                c.write_status(p,self.record(argv,code));self.assertEqual(c.read_status(p,argv)['result']['returncode'],code);p.unlink()
            for bad in [dict(valid,schema=True),dict(valid,extra=0),dict(valid,argv_sha256='0'*64),dict(valid,kind='unknown'),dict(valid,returncode=True),dict(valid,returncode=2**32),dict(valid,returncode=-2**31-1)]:
                p.write_text(json.dumps(bad),encoding='utf-8')
                with self.assertRaises(ValueError):c.read_status(p,argv)
            for raw in [b'[]',b'{"schema":1,"schema":1}',b'{"returncode":1e999}',b'{"returncode":NaN}']:
                p.write_bytes(raw)
                with self.assertRaises(ValueError):c.read_status(p,argv)
    def test_status_read_rejects_links_and_budget_before_open(self):
        c=r.child_status
        with tempfile.TemporaryDirectory() as d:
            p=Path(d)/'x.child-status.json';p.write_bytes(b'x'*(c.LIMIT+1));link=Path(d)/'link';link.symlink_to(p)
            with patch.object(Path,'open',side_effect=AssertionError('must not read')):
                for value in (p,link):
                    with self.assertRaises(ValueError):c.read_status(value,['a'])
    def fake_phase(self,work,argv,receipt,code=0,fail=None,write=True):
        c=r.child_status;holder=SimpleNamespace(__file__='old-worker')
        def phase(name,actual,w,env,report):
            self.assertEqual(holder.__file__,c.__file__);self.assertEqual(actual,argv)
            report['phases'].append({'name':name,'exit':0 if code==0 else 4294967295,'job_empty':True,'process_succeeded':code==0})
            if write:c.write_status(Path(env[c.STATUS_ENV]),self.record(actual,code))
            if fail is not None:raise fail
            return b'exact stdout'
        holder.phase=phase
        return holder
    def test_parent_success_preserves_stdout_and_restores_worker(self):
        c=r.child_status;argv=['fixed'];receipt={'phases':[]};env={'PATH':'fixed',c.STATUS_ENV:'foreign'};before=env.copy()
        with tempfile.TemporaryDirectory() as d:
            base=self.fake_phase(Path(d),argv,receipt)
            self.assertEqual(c.phase(base,'x',argv,Path(d),env,receipt),b'exact stdout')
            self.assertEqual(base.__file__,'old-worker');self.assertEqual(env,before)
            self.assertEqual(receipt['phases'][0]['child_status']['result']['returncode'],0)
    def test_failure_retains_wrapper_and_raw_status_without_retry(self):
        c=r.child_status;argv=['fixed'];receipt={'phases':[]};failure=RuntimeError('version failed')
        with tempfile.TemporaryDirectory() as d:
            base=self.fake_phase(Path(d),argv,receipt,code=0xc0000135,fail=failure)
            with self.assertRaises(RuntimeError) as caught:c.phase(base,'version',argv,Path(d),{},receipt)
            self.assertIs(caught.exception,failure);self.assertEqual(base.__file__,'old-worker')
            self.assertEqual(len(receipt['phases']),1);self.assertEqual(receipt['phases'][0]['exit'],4294967295)
            self.assertEqual(receipt['phases'][0]['child_status']['result']['returncode'],0xc0000135)
    def test_missing_status_cannot_create_success(self):
        c=r.child_status;argv=['fixed'];receipt={'phases':[]}
        with tempfile.TemporaryDirectory() as d:
            base=self.fake_phase(Path(d),argv,receipt,write=False)
            with self.assertRaises(FileNotFoundError):c.phase(base,'x',argv,Path(d),{},receipt)
            self.assertFalse(receipt['phases'][0]['process_succeeded']);self.assertIn('child_status_error',receipt['phases'][0]);self.assertEqual(base.__file__,'old-worker')
    def test_timeout_keeps_original_failure_and_missing_observation(self):
        c=r.child_status;argv=['fixed'];receipt={'phases':[]};failure=RuntimeError('Phase timeout')
        with tempfile.TemporaryDirectory() as d:
            base=self.fake_phase(Path(d),argv,receipt,fail=failure,write=False)
            with self.assertRaises(RuntimeError) as caught:c.phase(base,'x',argv,Path(d),{},receipt)
            self.assertIs(caught.exception,failure);self.assertIn('child_status_error',receipt['phases'][0]);self.assertEqual(base.__file__,'old-worker')
    def test_nonzero_raw_status_cannot_be_reported_success(self):
        c=r.child_status;argv=['fixed'];receipt={'phases':[]}
        with tempfile.TemporaryDirectory() as d:
            base=self.fake_phase(Path(d),argv,receipt,code=1)
            with self.assertRaisesRegex(RuntimeError,'Raw child'):c.phase(base,'x',argv,Path(d),{},receipt)
            self.assertFalse(receipt['phases'][0]['process_succeeded'])
    def test_adapter_adds_no_process_or_formatter_attempt(self):
        c=r.child_status;tree=ast.parse(Path(c.__file__).read_bytes())
        calls=[n for n in ast.walk(tree) if isinstance(n,ast.Call) and isinstance(n.func,ast.Attribute) and isinstance(n.func.value,ast.Name) and n.func.value.id=='subprocess']
        self.assertEqual([n.func.attr for n in calls],['run'])
        source=(HERE/'run.py').read_text(encoding='utf-8')
        self.assertEqual(source.count("child_status.phase(base,'version'"),1)
        self.assertEqual(r.probe.COUNTS,{'damage':3,'terrain-cell':4});self.assertIn('info.basic.active=3',(HERE/'winjob.py').read_text(encoding='utf-8'))
        self.assertEqual(r.base.TIMEOUT,120);self.assertEqual(r.base.LOG_LIMIT,128*1024)

if __name__=='__main__':unittest.main()
