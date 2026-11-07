"""No subprocess/network/native execution: fixed build preparation contracts only."""
import ast
import copy
import ctypes
import hashlib
import json
from pathlib import Path
import struct
import tempfile
import unittest
import zipfile
from unittest.mock import patch
import prepare as p
import run as r
import winjob
import inspect_pe

class Tests(unittest.TestCase):
    def setUp(self):
        self.tmp=tempfile.TemporaryDirectory();self.addCleanup(self.tmp.cleanup);self.root=Path(self.tmp.name)

    def test_unready_contract_cannot_start_any_phase(self):
        with self.assertRaises(ValueError):r.check_config({'ready':False})

    def test_exact_release_host_and_no_self_update_install(self):
        rustup=Path('/rustup.exe')
        self.assertEqual(p.install_argv(rustup),[str(rustup),'toolchain','install',p.TOOLCHAIN,'--profile','minimal','--component','rustfmt','--no-self-update'])

    def test_isolated_offline_env_discards_rustup_and_build_overrides(self):
        old={'PATH':'base','RUSTUP_HOME':'foreign','RUSTUP_DIST_SERVER':'https://other','RUSTUP_TOOLCHAIN':'stable','CARGO_BUILD_JOBS':'99'}
        saved=old.copy();env=p.environment(old,self.root)
        self.assertEqual(old,saved);self.assertEqual(env['CARGO_BUILD_JOBS'],'1');self.assertEqual(env['CARGO_NET_OFFLINE'],'true')
        self.assertEqual(env['RUSTUP_HOME'],str(self.root/'rustup'));self.assertEqual(env['RUSTUP_DIST_SERVER'],'https://static.rust-lang.org')
        self.assertEqual(env['RUSTUP_AUTO_INSTALL'],'0');self.assertNotIn('RUSTUP_TOOLCHAIN',env)

    def test_owned_job_process_cap_and_pid_buffer_match(self):
        source=(r.HERE/'winjob.py').read_text(encoding='utf-8')
        self.assertIn('info.basic.active=64',source);self.assertIn('8+64*C.sizeof(C.c_size_t)',source)
        self.assertIn('info.job_memory=10*1024**3',source);self.assertIn('0x2000|0x0008|0x0200',source)
        self.assertEqual(ctypes.sizeof(winjob.Basic),64);self.assertEqual(ctypes.sizeof(winjob.Limits),144)

    def test_phase_budgets_restore_on_failure(self):
        old=(r.base.TIMEOUT,r.base.LOG_LIMIT)
        with patch.object(r.status,'phase',side_effect=RuntimeError('stop')):
            with self.assertRaises(RuntimeError):r.phase('x',['a'],'build',self.root,{}, {'phases':[]})
        self.assertEqual((r.base.TIMEOUT,r.base.LOG_LIMIT),old)

    def test_low_disk_refuses_and_restores_resource_guard(self):
        old=r.base.disk_guard
        with patch.object(r.shutil,'disk_usage',return_value=type('U',(),{'free':r.base.FLOOR+r.RESERVE-1,'total':10**12})()):
            with self.assertRaises(RuntimeError):
                with r.bounded_resources({},[self.root]):pass
        self.assertIs(r.base.disk_guard,old)

    def test_low_ram_refuses_and_restores_resource_guard(self):
        old=r.base.disk_guard
        with patch.object(r.shutil,'disk_usage',return_value=type('U',(),{'free':10**12,'total':2*10**12})()),patch.object(r.resource,'memory',return_value={'available_physical':r.RAM_FLOOR-1}):
            with self.assertRaises(RuntimeError):
                with r.bounded_resources({},[self.root]):pass
        self.assertIs(r.base.disk_guard,old)

    def test_official_checkout_is_four_owned_commands_only(self):
        calls=[]
        with patch.object(r,'phase',side_effect=lambda *a:calls.append(a)):
            result=r.git_checkout('horizon',r.launch.HORIZON,self.root,'git.exe',{}, {})
        self.assertEqual(result,self.root/'horizon');self.assertEqual(len(calls),4)
        self.assertEqual(calls[1][1][-1],'https://github.com/horizon-rust/horizon.git')
        self.assertEqual(calls[2][1][-1],r.launch.HORIZON);self.assertEqual(calls[3][1][-2: ],['--detach',r.launch.HORIZON])

    def test_unknown_download_never_opens_network(self):
        with patch.object(p.urllib.request,'build_opener',side_effect=AssertionError('network')):
            with self.assertRaises(ValueError):p.download('https://other',self.root/'x','0'*64,1)

    def test_winlibs_fresh_safe_prefix_and_complete_files(self):
        archive=self.root/'winlibs.zip';toolchain=self.root/'tc';toolchain.mkdir()
        with zipfile.ZipFile(archive,'w') as z:
            z.writestr('mingw64/bin/gcc.exe',b'fake');z.writestr('mingw64/lib/empty',b'')
        with patch.object(p,'record',return_value={'sha256':p.WINLIBS_SHA,'bytes':archive.stat().st_size}):
            result=p.unpack_winlibs(archive,toolchain)
            self.assertEqual(result['files'],2);self.assertEqual(result['expanded_bytes'],4)
            with self.assertRaises(ValueError):p.unpack_winlibs(archive,toolchain)

    def test_winlibs_unsafe_case_and_link_rejected_before_writing(self):
        cases=[['other/bin/a'],['mingw64/../a'],['mingw64/a','mingw64/A']]
        for i,names in enumerate(cases):
            archive=self.root/f'bad{i}.zip';tc=self.root/f'tc{i}';tc.mkdir()
            with zipfile.ZipFile(archive,'w') as z:
                for name in names:z.writestr(name,b'x')
            with patch.object(p,'record',return_value={'sha256':p.WINLIBS_SHA,'bytes':archive.stat().st_size}):
                with self.assertRaises(ValueError):p.unpack_winlibs(archive,tc)
            self.assertEqual(list(tc.iterdir()),[])
        archive=self.root/'link.zip';tc=self.root/'linktc';tc.mkdir()
        with zipfile.ZipFile(archive,'w') as z:
            e=zipfile.ZipInfo('mingw64/link');e.external_attr=0o120777<<16;z.writestr(e,b'target')
        with patch.object(p,'record',return_value={'sha256':p.WINLIBS_SHA,'bytes':archive.stat().st_size}):
            with self.assertRaises(ValueError):p.unpack_winlibs(archive,tc)
        self.assertEqual(list(tc.iterdir()),[])

    def test_component_manifest_hash_and_installed_set_are_exact(self):
        root=self.root/'tc';d=root/'lib/rustlib';d.mkdir(parents=True)
        def content(drift=False):
            text='manifest-version = "2"\ndate = "2026-10-15"\n'
            for name,h in p.COMPONENTS.items():
                dist='rustfmt' if name=='rustfmt-preview' else name
                text+=f'[pkg.{name}.target.{r.launch.TARGET}]\navailable = true\nhash = "{("0"*64) if drift else h}"\nurl = "https://static.rust-lang.org/dist/2026-10-15/{dist}-1.99.1-{r.launch.TARGET}.tar.gz"\n'
            return text
        path=d/'multirust-channel-manifest.toml';path.write_text(content(),encoding='utf-8')
        # Independent rustup 1.29.2 display contract, not derived from manifest keys.
        names=['cargo-x86_64-pc-windows-gnu','rust-mingw-x86_64-pc-windows-gnu',
               'rust-std-x86_64-pc-windows-gnu','rustc-x86_64-pc-windows-gnu',
               'rustfmt-x86_64-pc-windows-gnu']
        installed='\n'.join(names)
        self.assertEqual(p.admit_manifest(root,installed)['components'],sorted(names))
        with self.assertRaises(ValueError):
            p.admit_manifest(root,installed.replace('rustfmt-','rustfmt-preview-'))
        for value in (installed+'\nclippy',installed.split('\n',1)[1],installed+'\n'+installed.splitlines()[0]):
            with self.assertRaises(ValueError):p.admit_manifest(root,value)
        path.write_text(content(True),encoding='utf-8')
        with self.assertRaises(ValueError):p.admit_manifest(root,installed)

    def test_official_lock_preserved_and_only_plan_local_packages_added(self):
        original=self.root/'old';updated=self.root/'new';plan=self.root/'plan'
        old='version = 4\n[[package]]\nname = "base"\nversion = "1.0.0"\nsource = "registry+official"\nchecksum = "abc"\n'
        new=old+'[[package]]\nname = "mod"\nversion = "0.1.0"\n'
        original.write_text(old);updated.write_text(new);plan.write_text(json.dumps({'crates':[{'package':'mod','version':'0.1.0'}]}))
        self.assertEqual(r.verify_lock_diff(original,updated,plan)['added_local_packages'],[{'name':'mod','version':'0.1.0','source':None}])
        for bad in (new.replace('checksum = "abc"','checksum = "bad"'),new+'source = "registry+extra"\n',old):
            updated.write_text(bad)
            with self.assertRaises(ValueError):r.verify_lock_diff(original,updated,plan)

    def test_link_result_requires_exact_mode_hash_and_contained_output(self):
        home=self.root/'home';build=home/'builds-latest-gnu'/'key';build.mkdir(parents=True)
        exe=build/'horizon-client.exe';exe.write_bytes(b'fake PE bytes');h=p.record(exe)['sha256']
        ready=build/'.linked.json';ready.write_text(json.dumps({'inputs':{'mode':'link-only','resources':None},'outputs':{'game':h}}))
        out={'scope':'guarded-windows-link-only','game_started':False,'resources_prepared':False,'receipt':str(ready),'executable':str(exe),'sha256':h}
        r.linked_result(json.dumps(out).encode(),home)
        for key,value in [('game_started',True),('sha256','0'*64),('executable',str(self.root/'outside.exe'))]:
            bad=dict(out);bad[key]=value
            with self.assertRaises(ValueError):r.linked_result(json.dumps(bad).encode(),home)

    def test_pe_rejects_truncation_and_wrong_machine(self):
        for raw in (b'',b'MZ',b'MZ'+bytes(100)):
            with self.assertRaises((ValueError,struct.error)):inspect_pe.inspect(raw)

    def test_existing_phase_and_child_status_are_not_reimplemented(self):
        source=(r.HERE/'run.py').read_text(encoding='utf-8')
        self.assertIn("HERE.parent/'windows-source-ci/run.py'",source)
        self.assertIn("HERE.parent/'windows-formatter-ci/child_status.py'",source)
        self.assertNotIn('subprocess.Popen',source);self.assertNotIn('subprocess.run',source)

    def test_fresh_ordinary_build_is_once_and_help_is_later(self):
        source=(r.HERE/'run.py').read_text(encoding='utf-8')
        self.assertEqual(source.count("phase('build-only'"),1)
        self.assertLess(source.index("phase('build-only'"),source.index("phase('imports'"))
        self.assertLess(source.index("phase('imports'"),source.index("phase('help'"))
        self.assertIn("'--build-only'",source);self.assertNotIn("'--no-default-features'",source)
        self.assertIn("online['CARGO_NET_OFFLINE']='false'",source)
        self.assertIn("'fetch','--locked'",source)
        self.assertNotIn("'build','--offline'",source) # no alternative compiler route

if __name__=='__main__':unittest.main()
