"""Pure/mock only. No downloads, Windows API, formatter or subprocess is run."""
import ctypes
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
        self.assertLess(run.index("base.phase('stat-pure'"),run.index("phase_json('prepare'"))
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
if __name__=='__main__':unittest.main()
