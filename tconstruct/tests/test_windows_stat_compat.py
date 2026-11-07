"""Pure cross-API Windows stat model and file-backed reads; no native tools."""
import hashlib
import json
import os
from pathlib import Path
import stat
import sys
import tempfile
from types import SimpleNamespace
import unittest
from windows_source_layers import before_package_contract
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT/'tools'))
import playtest_pluto_inputs as p
import playtest_guard_rustfmt as f
import play_windows_latest as w


def info(value=None, **changes):
    values = dict(st_dev=11, st_ino=22, st_mode=stat.S_IFREG | 0o666,
                  st_size=4, st_mtime_ns=33, st_nlink=1,
                  st_file_attributes=0, st_reparse_tag=0)
    if value is not None:
        values.update({k:getattr(value,k,0) for k in values})
    values.update(changes)
    return SimpleNamespace(**values)


class Tests(unittest.TestCase):
    def test_known_windows_extensions_only_add_filename_execute_bits(self):
        handle = info(); before = info(handle, st_mode=handle.st_mode | 0o111)
        with patch.object(p, 'WINDOWS_STAT', True):
            for name in ('a.exe','a.EXE','a.bat','a.cmd','a.com'):
                p.require_opened_file(Path(name), before, handle)
        self.assertNotEqual(p.signature(before),p.signature(handle))

    def test_posix_preserves_exact_mode_comparison(self):
        h=info()
        with patch.object(p,'WINDOWS_STAT',False):
            p.require_opened_file(Path('a.exe'),h,h)
            with self.assertRaises(ValueError):p.require_opened_file(Path('a.exe'),info(h,st_mode=h.st_mode|0o111),h)

    def test_unknown_extension_and_nonexecute_modes_reject(self):
        h=info(); b=info(h,st_mode=h.st_mode|0o111)
        with patch.object(p,'WINDOWS_STAT',True):
            for path in ('a.rs','a.dll','a.exe.txt','a'):
                with self.assertRaises(ValueError):p.require_opened_file(Path(path),b,h)
            with self.assertRaises(ValueError):p.require_opened_file(Path('a.exe'),info(b,st_mode=b.st_mode^0o200),h)

    def test_cross_api_keeps_every_other_identity_field(self):
        h=info(); b=info(h,st_mode=h.st_mode|0o111)
        with patch.object(p,'WINDOWS_STAT',True):
            for key in ('st_dev','st_ino','st_size','st_mtime_ns','st_nlink','st_file_attributes','st_reparse_tag'):
                with self.subTest(key=key),self.assertRaises(ValueError):
                    p.require_opened_file(Path('a.exe'),info(b,**{key:getattr(b,key)+1}),h)

    def test_file_type_and_reparse_are_not_normalized(self):
        h=info()
        with patch.object(p,'WINDOWS_STAT',True):
            for changed in (info(h,st_mode=stat.S_IFDIR|0o777),info(h,st_mode=stat.S_IFLNK|0o777),info(h,st_file_attributes=p.REPARSE)):
                with self.assertRaises(ValueError):p.require_opened_file(Path('a.exe'),changed,h)

    def exercise(self, reader, final_change=None, handle_change=None):
        with tempfile.TemporaryDirectory() as td:
            target=Path(td)/'tool.exe';target.write_bytes(b'1234')
            original_lstat=Path.lstat
            raw=target.lstat();handle=info(raw,st_mode=stat.S_IFREG|0o666)
            before=info(handle,st_mode=handle.st_mode|0o111)
            count=0
            def path_stat(path,*args,**kwargs):
                nonlocal count
                if path==target:
                    count+=1
                    return info(before,**(final_change or {})) if count>1 else before
                return original_lstat(path,*args,**kwargs)
            with patch.object(p,'WINDOWS_STAT',True),patch.object(Path,'lstat',path_stat),patch.object(os,'fstat',side_effect=[handle,info(handle,**(handle_change or {}))]):
                if reader=='formatter':return f.bounded_file(target,10)
                return (p if reader=='cache' else w).file_hash(target,target.lstat())

    def test_three_read_paths_accept_mode_difference_without_execution(self):
        self.assertEqual(self.exercise('formatter'),b'1234')
        for name in ('cache','launcher'):
            self.assertEqual(self.exercise(name),(hashlib.sha256(b'1234').hexdigest(),4))

    def test_path_final_execute_bits_change_still_rejects(self):
        for name in ('formatter','cache','launcher'):
            with self.subTest(name=name),self.assertRaises(ValueError):
                self.exercise(name,final_change={'st_mode':stat.S_IFREG|0o666})

    def test_handle_after_execute_bits_change_still_rejects(self):
        for name in ('formatter','cache','launcher'):
            with self.subTest(name=name),self.assertRaises(ValueError):
                self.exercise(name,handle_change={'st_mode':stat.S_IFREG|0o777})

    def test_handle_drift_nlink_and_reparse_still_reject(self):
        for name in ('formatter','cache','launcher'):
            for change in ({'st_nlink':2},{'st_ino':999},{'st_file_attributes':p.REPARSE}):
                with self.subTest(name=name,change=change),self.assertRaises(ValueError):self.exercise(name,handle_change=change)

    def test_path_after_identity_drift_still_rejects(self):
        for name in ('formatter','cache','launcher'):
            for change in ({'st_mtime_ns':999},{'st_size':6},{'st_nlink':2}):
                with self.subTest(name=name,change=change),self.assertRaises(ValueError):self.exercise(name,final_change=change)

    def test_regular_real_bounded_read_and_limits(self):
        with tempfile.TemporaryDirectory() as td:
            path=Path(td)/'input.bin';path.write_bytes(b'1234')
            self.assertEqual(f.bounded_file(path,4),b'1234')
            with self.assertRaises(ValueError):f.bounded_file(path,3)

    def test_source_changes_are_exactly_reversible(self):
        contract=json.loads((ROOT/'experiments/windows-stat-compat/inputs.json').read_bytes())
        for name,row in contract['files'].items():
            raw=(ROOT/'tools'/name).read_bytes()
            raw=before_package_contract('tools/'+name,raw)
            self.assertEqual(hashlib.sha256(raw).hexdigest(),row['after_sha256'])
            text=raw.decode()
            for old,new in reversed(row['edits']):
                self.assertEqual(text.count(new),1)
                text=text.replace(new,old,1)
            self.assertEqual(hashlib.sha256(text.encode()).hexdigest(),row['before_sha256'])


if __name__=='__main__':unittest.main()
