"""Synthetic archives and mocked resource/network observations; never native."""
import ctypes
import hashlib
import io
import json
import os
from pathlib import Path
from types import SimpleNamespace
import sys
import tarfile
import tempfile
import unittest
from unittest.mock import patch, Mock

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import rustc_runtime as r


class Tests(unittest.TestCase):
    def archive(self, path, extra=(), drop=None, commit=None):
        rows = [(r.TOP+'/git-commit-hash', (commit or r.COMMIT).encode(), 'file')]
        rows += [(r.TOP+'/rustc/bin/'+name, b'pinned synthetic member', 'file') for name in ('rustc.exe','rustdoc.exe',*r.REQUIRED_DLLS) if name != drop]
        rows += [(r.TOP+'/rustc/share/empty', b'', 'file'), *extra]
        with tarfile.open(path, 'w:gz') as t:
            for name, data, kind in rows:
                item = tarfile.TarInfo(name); item.size = len(data)
                if kind == 'link': item.type = tarfile.SYMTYPE; item.linkname = 'elsewhere'; item.size = 0
                t.addfile(item, io.BytesIO(data) if kind == 'file' else None)
        return hashlib.sha256(path.read_bytes()).hexdigest()

    def test_exact_rustc_source_pin_and_only_one_new_component(self):
        self.assertEqual(r.URL,'https://static.rust-lang.org/dist/2026-10-15/rustc-1.99.1-x86_64-pc-windows-gnu.tar.gz')
        self.assertEqual(r.SHA256,'43b17e9a9134ea72562bfbb23728a830f671ea9be1db6a72597a8d240e986b35')
        self.assertEqual(r.COMMIT,'58cf8f9c35b355199727627d07ae34a4e2846d67')
        self.assertNotIn('subprocess',Path(r.__file__).read_text(encoding='utf-8').replace('No installer, subprocess,',''))

    def test_full_component_payload_and_all_dlls_are_preserved(self):
        with tempfile.TemporaryDirectory() as d:
            p=Path(d);h=self.archive(p/'a',extra=[(r.TOP+'/rustc/bin/another.dll',b'other', 'file')])
            with patch.object(r,'SHA256',h): result=r.unpack(p/'a',p/'prefix')
            self.assertEqual(result['files'],r.seal(p/'prefix'))
            self.assertIn('bin/another.dll',result['files']);self.assertEqual(result['files']['share/empty']['bytes'],0)
            self.assertFalse(result['transitive_pe_closure_verified'])

    def test_digest_rejected_before_unpack(self):
        with tempfile.TemporaryDirectory() as d:
            p=Path(d);(p/'a').write_bytes(b'wrong')
            with patch.object(r.tarfile,'open') as unpack:
                with self.assertRaisesRegex(ValueError,'archive differs'):r.unpack(p/'a',p/'out')
                unpack.assert_not_called()
            self.assertFalse((p/'out').exists())

    def test_missing_dependency_and_compiler_commit_rejected(self):
        for missing,commit in [(r.REQUIRED_DLLS[0],None),(None,'0'*40)]:
            with self.subTest(missing=missing),tempfile.TemporaryDirectory() as d:
                p=Path(d);h=self.archive(p/'a',drop=missing,commit=commit)
                with patch.object(r,'SHA256',h),self.assertRaises(ValueError):r.unpack(p/'a',p/'out')

    def test_duplicate_case_parent_and_symlink_rejected(self):
        for suffix,kind in [('rustc/bin/RUSTC.EXE','file'),('rustc/../outside','file'),('rustc/bin/link.dll','link')]:
            with self.subTest(suffix=suffix),tempfile.TemporaryDirectory() as d:
                p=Path(d);h=self.archive(p/'a',[(r.TOP+'/'+suffix,b'bad',kind)])
                with patch.object(r,'SHA256',h),self.assertRaises(ValueError):r.unpack(p/'a',p/'out')

    def test_windows_reserved_paths_and_count_bounds(self):
        for name in ('/absolute','a//b','a/CON.txt','a/trailing.','a/back\\slash','a/x:y','a/../z','a/'+'x'*513):
            with self.subTest(name=name),self.assertRaises(ValueError):r.safe_name(name)
        with tempfile.TemporaryDirectory() as d:
            p=Path(d);h=self.archive(p/'a')
            with patch.object(r,'SHA256',h),patch.object(r,'MEMBER_LIMIT',1),self.assertRaises(ValueError):r.unpack(p/'a',p/'out')

    def test_expansion_and_individual_file_limits(self):
        for name in ('EXPANDED_LIMIT','FILE_LIMIT'):
            with self.subTest(name=name),tempfile.TemporaryDirectory() as d:
                p=Path(d);h=self.archive(p/'a')
                with patch.object(r,'SHA256',h),patch.object(r,name,1),self.assertRaises(ValueError):r.unpack(p/'a',p/'out')

    def test_decompressed_headers_are_bounded_too(self):
        with patch.object(r,'EXPANDED_LIMIT',1),patch.object(r,'MEMBER_LIMIT',0):
            reader=r.ExpandedReader(io.BytesIO(b'xx'))
            with self.assertRaisesRegex(ValueError,'stream budget'):reader.read(100)
            with self.assertRaises(ValueError):r.ExpandedReader(io.BytesIO(b'')).read(-1)

    def test_fresh_destination_and_parent_links_rejected(self):
        with tempfile.TemporaryDirectory() as d:
            p=Path(d);h=self.archive(p/'a');(p/'out').mkdir();(p/'link').symlink_to(p/'out',target_is_directory=True)
            with patch.object(r,'SHA256',h):
                for target in (p/'out',p/'link'/'new'):
                    with self.assertRaises(ValueError):r.unpack(p/'a',target)

    def test_streaming_record_empty_and_budget_and_link(self):
        with tempfile.TemporaryDirectory() as d:
            p=Path(d)/'a';p.write_bytes(b'');self.assertEqual(r.file_record(p)['bytes'],0)
            p.write_bytes(b'123');link=p.with_name('link');link.symlink_to(p)
            with self.assertRaises(ValueError):r.file_record(p,2)
            with self.assertRaises(ValueError):r.file_record(link)
            self.assertEqual(r.file_record(p),{'bytes':3,'sha256':hashlib.sha256(b'123').hexdigest()})

    def test_download_has_one_fixed_origin_and_hash_readback(self):
        payload=b'archive';response=Mock();response.status=200;response.geturl.return_value=r.URL;response.headers={'Content-Length':str(len(payload))};response.read.side_effect=[payload,b''];response.__enter__=Mock(return_value=response);response.__exit__=Mock(return_value=False)
        opener=Mock();opener.open.return_value=response
        with tempfile.TemporaryDirectory() as d,patch.object(r.urllib.request,'build_opener',return_value=opener),patch.object(r,'SHA256',hashlib.sha256(payload).hexdigest()):
            result=r.download(Path(d)/'a');self.assertEqual(result['bytes'],len(payload));opener.open.assert_called_once_with(r.URL,timeout=15)

    def test_download_origin_and_size_fail_closed(self):
        for url,length in [(r.URL+'?other','1'),(r.URL,str(r.ARCHIVE_LIMIT+1))]:
            response=Mock();response.status=200;response.geturl.return_value=url;response.headers={'Content-Length':length};response.__enter__=Mock(return_value=response);response.__exit__=Mock(return_value=False)
            opener=Mock();opener.open.return_value=response
            with tempfile.TemporaryDirectory() as d,patch.object(r.urllib.request,'build_opener',return_value=opener),self.assertRaises(ValueError):r.download(Path(d)/'a')
            response.read.assert_not_called()

    def test_memory_layout_and_floor_restores_original_guard(self):
        self.assertEqual(ctypes.sizeof(r.MemoryStatus),64)
        original=Mock();base=SimpleNamespace(disk_guard=original);receipt={}
        sample={'total_physical':2**30,'available_physical':r.RAM_FLOOR-1,'total_pagefile':2**31,'available_pagefile':2**30}
        with patch.object(r,'memory',return_value=sample),patch.object(r.shutil,'disk_usage',return_value=SimpleNamespace(total=100,free=90)):
            with self.assertRaisesRegex(RuntimeError,'RAM floor'):
                with r.observed_guard(base,receipt):base.disk_guard([Path('/tmp')])
        self.assertIs(base.disk_guard,original);self.assertEqual(receipt['resources']['memory_samples'],1)

    def test_actual_numeric_resource_minima_are_bounded_aggregate(self):
        original=Mock();base=SimpleNamespace(disk_guard=original);receipt={}
        samples=[{'total_physical':4*1024**3,'available_physical':v,'total_pagefile':8*1024**3,'available_pagefile':v} for v in (2*1024**3,1024**3)]
        with patch.object(r,'memory',side_effect=samples),patch.object(r.shutil,'disk_usage',side_effect=[SimpleNamespace(total=100,free=90),SimpleNamespace(total=100,free=80)]):
            with r.observed_guard(base,receipt):base.disk_guard([Path('/tmp')]);base.disk_guard([Path('/tmp')])
        self.assertEqual(receipt['resources']['minimum_available_physical_bytes'],1024**3)
        self.assertEqual(receipt['resources']['filesystems'][str(Path('/tmp').anchor)]['minimum_free_bytes'],80)
        self.assertEqual(original.call_count,2)

    def test_original_probe_and_controller_whole_bytes_invert(self):
        rows=json.loads((HERE/'runtime-inverse.json').read_bytes())
        for name,expected in [('probe.py','e15d6b817e6e01bb1c295faf109b3096cb9832632e7361cf0286c7575a101527'),('run.py','a99ec5bacf2b477918af25202a595b431c01a349477a15e0d0062ec325ba4f01'),('test_runner.py','37c08a9f09461309c831269a132738ac7f7cada517b6ae5f269c76359786368c')]:
            source=(HERE/name).read_text(encoding='utf-8')
            for row in reversed(rows[name]):
                self.assertEqual(source.count(row['after']),1);source=source.replace(row['after'],row['before'],1)
            self.assertEqual(hashlib.sha256(source.encode()).hexdigest(),expected)

    def test_runtime_precedes_formatter_and_original_limits_stay(self):
        source=(HERE/'run.py').read_text(encoding='utf-8')
        self.assertLess(source.index("phase_json('rustc-download'"),source.index("phase_json('rustc-runtime'"))
        self.assertLess(source.index("phase_json('rustc-runtime'"),source.index("phase_json('prepare'"))
        self.assertLess(source.index("phase_json('prepare'"),source.index("child_status.phase(base,'version'"))
        self.assertEqual(source.count("child_status.phase(base,'version'"),1)
        self.assertIn('base.TIMEOUT=120',source)
        self.assertIn('info.basic.active=3',(HERE/'winjob.py').read_text(encoding='utf-8'))
        self.assertIn('guard.verify_source(plan,official,guard.entries())',(HERE/'probe.py').read_text(encoding='utf-8'))


if __name__ == '__main__': unittest.main()
