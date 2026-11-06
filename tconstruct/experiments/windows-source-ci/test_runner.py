"""Pure parser/source regressions; does not use Windows APIs or run native code."""
import ast
import ctypes
import importlib.util
from pathlib import Path
import subprocess
import sys
import unittest
import tempfile
from unittest.mock import patch, Mock, MagicMock

HERE=Path(__file__).resolve().parent
sys.path.insert(0,str(HERE))
import validate
import winjob
import run as controller


def output(names):
    return (''.join(f'{n} (__main__.Tests.{n}) ... ok\n' for n in sorted(names))+'\n'+'-'*70+'\n'+f'Ran {len(names)} tests in 0.012s\n\nOK\n').encode()

class Tests(unittest.TestCase):
    def test_exact_names_and_crlf(self):
        raw=output(['test_a','test_b']);self.assertEqual(validate.suite(raw,['test_b','test_a'])['tests'],2)
        self.assertEqual(validate.suite(raw.replace(b'\n',b'\r\n'),['test_a','test_b'])['passed'],2)
    def test_warning_duplicate_missing_extra_reject(self):
        raw=output(['test_a','test_b'])
        for changed in (b'warning\n'+raw,raw+raw,output(['test_a']),output(['test_a','test_a']),output(['test_a','test_b','test_c'])):
            with self.assertRaises(ValueError):validate.suite(changed,['test_a','test_b'])
    def test_zero_skip_failure_and_wrong_count_reject(self):
        for changed in (output([]),output(['test_a']).replace(b'... ok',b'... skipped reason'),output(['test_a']).replace(b'OK',b'FAILED'),output(['test_a']).replace(b'Ran 1 tests',b'Ran 2 tests')):
            with self.assertRaises(ValueError):validate.suite(changed,['test_a'])
    def test_ps_parser_no_execution_token(self):
        self.assertFalse(validate.powershell(b'PS_PARSE_OK\r\n')['executed_entry'])
        for raw in (b'',b'PS_PARSE_OK\nwarning\n',b'error\n'):
            with self.assertRaises(ValueError):validate.powershell(raw)
    def test_windows_layout_and_barrier_order(self):
        self.assertEqual(ctypes.sizeof(winjob.Basic),64);self.assertEqual(ctypes.sizeof(winjob.Limits),144)
        s=(HERE/'run.py').read_text()
        self.assertLess(s.index('job.assign(process)'),s.index("process.stdin.write(b'G')"))
        self.assertLess(s.index("sys.stdin.buffer.read(1)"),s.index("return subprocess.run(sys.argv[2:]"))
        self.assertIn('job.wait_empty(guard)',s)
    def test_low_disk_never_starts_worker(self):
        with tempfile.TemporaryDirectory() as temp:
            receipt={'phases':[]}
            with patch.object(controller,'disk_guard',side_effect=RuntimeError('floor')), patch.object(controller.subprocess,'Popen') as spawn, patch.object(winjob,'Job') as job:
                with self.assertRaisesRegex(RuntimeError,'floor'):controller.phase('pure',['python','known.py'],Path(temp),{},receipt)
                spawn.assert_not_called();job.assert_not_called()
            self.assertFalse(receipt['phases'][0]['process_succeeded'])
    def test_failed_job_assignment_keeps_barrier_and_kills_only_worker(self):
        with tempfile.TemporaryDirectory() as temp:
            process=Mock();job=Mock();job.assign.side_effect=RuntimeError('assignment refused')
            with patch.object(controller,'disk_guard'), patch.object(winjob,'Job',return_value=job), patch.object(controller.subprocess,'Popen',return_value=process), patch.object(controller.subprocess,'CREATE_NEW_PROCESS_GROUP',0,create=True):
                with self.assertRaisesRegex(RuntimeError,'assignment refused'):controller.phase('pure',['python','known.py'],Path(temp),{}, {'phases':[]})
            process.stdin.write.assert_not_called();process.kill.assert_called_once();process.wait.assert_called_once_with(timeout=2)
            job.terminate.assert_not_called();job.close.assert_called_once()
    def test_handle_assignment_and_normal_cleanup_are_required(self):
        s=(HERE/'winjob.py').read_text();self.assertIn('int(process._handle)',s);self.assertNotIn('OpenProcess',s)
        with tempfile.TemporaryDirectory() as temp:
            process=Mock();process.poll.return_value=0;process.returncode=0;job=Mock();receipt={'phases':[]}
            with patch.object(controller,'disk_guard'), patch.object(winjob,'Job',return_value=job), patch.object(controller.subprocess,'Popen',return_value=process), patch.object(controller.subprocess,'CREATE_NEW_PROCESS_GROUP',0,create=True):
                self.assertEqual(controller.phase('pure',['python','known.py'],Path(temp),{},receipt),b'')
            job.assign.assert_called_once_with(process);process.stdin.write.assert_called_once_with(b'G')
            job.terminate.assert_called_once();job.wait_empty.assert_called_once();job.close.assert_called_once()
            self.assertTrue(receipt['phases'][0]['job_empty'])
    def test_fixed_three_suite_counts_paths_and_order(self):
        import copy
        groups=[{'name':name,'path':path,'names':['test_'+str(x) for x in range(n)]} for name,path,n in controller.SUITES]
        controller.check_suites(groups)
        bad=copy.deepcopy(groups);bad[0]['names']=bad[0]['names'][:-1]
        wrong=copy.deepcopy(groups);wrong[0]['path']='other.py'
        duplicate=copy.deepcopy(groups);duplicate[0]['names'][0]=duplicate[0]['names'][1]
        for value in (groups[::-1],groups[:-1],bad,wrong,duplicate):
            with self.assertRaises(ValueError):controller.check_suites(value)
    def test_final_log_limit_precedes_bounded_read(self):
        path=MagicMock();path.stat.return_value.st_size=controller.LOG_LIMIT+1
        with self.assertRaisesRegex(RuntimeError,'log limit'):controller.read_log(path)
        path.open.assert_not_called()
        path=MagicMock();path.stat.return_value.st_size=1
        stream=path.open.return_value.__enter__.return_value;stream.read.return_value=b'x'*(controller.LOG_LIMIT+1)
        with self.assertRaisesRegex(RuntimeError,'log limit'):controller.read_log(path)
        stream.read.assert_called_once_with(controller.LOG_LIMIT+1)
        with tempfile.TemporaryDirectory() as temp:
            good=Path(temp)/'phase.log';good.write_bytes(b'bounded')
            self.assertEqual(controller.read_log(good),b'bounded')
    def test_native_and_installer_absent(self):
        s=(HERE/'run.py').read_text();tree=ast.parse(s)
        literals=[n.value for n in ast.walk(tree) if isinstance(n,ast.Constant) and isinstance(n.value,str)]
        self.assertNotIn('cargo',literals);self.assertNotIn('rustup',literals)
        self.assertNotIn('install',literals)
        self.assertIn("'native_executed':False",s)
    def test_actual_verbose_merged_suites_match_validator(self):
        # Source-stage regression root is explicit; no toolchain/metadata/game calls.
        import os
        root=Path(os.environ['WINDOWS_PURE_SOURCE_ROOT'])
        for filename,count in [('test_windows_latest_launch.py',20),('test_windows_pluto_cache.py',12),('test_windows_playtest_bundles.py',11)]:
            path=root/'tconstruct/tests'/filename
            parsed=ast.parse(path.read_text())
            names=[n.name for c in parsed.body if isinstance(c,ast.ClassDef) and c.name=='Tests' for n in c.body if isinstance(n,ast.FunctionDef) and n.name.startswith('test_')]
            self.assertEqual(len(names),count)
            r=subprocess.run([sys.executable,str(path),'-v'],stdout=subprocess.PIPE,stderr=subprocess.STDOUT,check=True,env={**os.environ,'PYTHONDONTWRITEBYTECODE':'1'},timeout=15)
            self.assertEqual(validate.suite(r.stdout,names)['passed'],count)

if __name__=='__main__':unittest.main()
