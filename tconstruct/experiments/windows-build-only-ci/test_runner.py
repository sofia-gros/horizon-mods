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

    def test_rustup_known_info_order_and_single_exact_version(self):
        version='rustup 1.29.3 (edbc831ce 2026-11-03)' # synthetic line, not recovered job bytes
        info=['info: This is the version for the rustup toolchain manager, not the rustc compiler.',
              'info: no `rustc` is currently active']
        for rows in ([version,*info],[info[0],version,info[1]],[*info,version]):
            for ending in ('\n','\r\n'):
                raw=(ending.join(rows)+ending).encode();receipt={}
                self.assertEqual(p.rustup_version(raw,receipt),version)
                self.assertEqual(receipt['rustup_observation']['output_utf8'],raw.decode())
                self.assertEqual(receipt['rustup_observation']['sha256'],hashlib.sha256(raw).hexdigest())
        raw=('\n'.join([version,*info])+'\n').encode()
        for bad in (raw.replace(b'1.29.3',b'1.29.2'),raw.replace(b'1.29.3',b'1.29.4'),
                    raw+version.encode()+b'\n',raw+b'unknown\n',b'\n'.join([version.encode(),info[0].encode()])):
            receipt={}
            with self.assertRaises(ValueError):p.rustup_version(bad,receipt)
            self.assertEqual(receipt['rustup_observation']['bytes'],len(bad))
            self.assertEqual(receipt['rustup_observation']['output_utf8'],bad.decode())

    def test_rustup_raw_observation_precedes_bounded_admission(self):
        for raw in (b'x'*4097,b'\xff'):
            receipt={}
            with self.assertRaises(ValueError):p.rustup_version(raw,receipt)
            observed=receipt['rustup_observation']
            self.assertEqual(observed['bytes'],len(raw))
            self.assertEqual(observed['sha256'],hashlib.sha256(raw).hexdigest())
            if len(raw)>4096:self.assertNotIn('output_utf8',observed)
        source=(r.HERE/'run.py').read_text(encoding='utf-8')
        self.assertEqual(source.count("phase('rustup-version'"),1)
        self.assertNotIn("rv.startswith('rustup 1.29.2 '",source)

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

    def repair_fixture(self):
        home=self.root/'home';stage=home/'builds-latest-gnu'/('a'*64)/'stage';stage.mkdir(parents=True)
        target='crates/horizon-world/src/worldgen/structure/start.rs'
        before=b'if !(0...=1.)';offset=len(b'if !(0.');after=before[:offset]+b' '+before[offset:]
        path=stage/'modded'/target;path.parent.mkdir(parents=True);path.write_bytes(after)
        plan=stage/'complete-plan.json';plan.write_bytes(b'{}')
        value={'schema':1,'kind':'known-float-range-token-boundary','target':target,
          'horizon':r.launch.HORIZON,'mixins':r.launch.MIXINS,'inserted_bytes':1,
          'inverse_verified':True,'other_stage_entries_unchanged':True,'original_and_locks_unchanged':True,
          'ast_verified':False,'semantic_change':False,'repair_applied':True,'offset':offset,
          'before_bytes':len(before),'after_bytes':len(after),'before_sha256':p.record(path)['sha256'],
          'after_sha256':p.record(path)['sha256'],'plan_sha256':p.record(plan)['sha256'],
          'stage_before_seal_sha256':'b'*64,'stage_after_seal_sha256':'c'*64}
        value['before_sha256']=hashlib.sha256(before).hexdigest()
        receipt=stage/'float-range-repair-receipt.json';receipt.write_text(json.dumps(value),encoding='utf-8')
        return home,stage,receipt,value

    def test_repair_receipt_observed_with_actual_file_inverse(self):
        home,stage,path,value=self.repair_fixture();receipt={};r.observe_float_repair(home,receipt)
        result=receipt['float_range_repair'];self.assertEqual(result['status'],'observed')
        self.assertEqual(result['result'],value);self.assertEqual(result['sha256'],p.record(path)['sha256'])
        self.assertFalse(result['result']['ast_verified'])

    def test_repair_absence_and_rejection_do_not_overwrite_build_error(self):
        receipt={'error':'original compile failure'};r.observe_float_repair(self.root/'absent',receipt)
        self.assertEqual(receipt['float_range_repair']['status'],'absent')
        home,stage,path,value=self.repair_fixture();path.write_bytes(b'bad JSON')
        r.observe_float_repair(home,receipt);self.assertEqual(receipt['float_range_repair']['status'],'rejected')
        self.assertEqual(receipt['error'],'original compile failure')
        self.assertLessEqual(len(receipt['float_range_repair']['error']),512)

    def test_repair_status_lengths_hashes_and_changed_plan_are_rejected(self):
        home,stage,path,value=self.repair_fixture()
        for key,bad in [('schema',True),('ast_verified',True),('inserted_bytes',2),('offset',True),
                        ('offset',999),('before_sha256','0'*64),('after_bytes',0),('target','../escape')]:
            path.write_text(json.dumps(dict(value,**{key:bad})),encoding='utf-8');receipt={}
            r.observe_float_repair(home,receipt);self.assertEqual(receipt['float_range_repair']['status'],'rejected')
        path.write_text(json.dumps(value),encoding='utf-8');(stage/'complete-plan.json').write_bytes(b'changed')
        receipt={};r.observe_float_repair(home,receipt);self.assertEqual(receipt['float_range_repair']['status'],'rejected')

    def test_repair_receipt_budget_duplicates_links_and_multiple_roots_refuse(self):
        home,stage,path,value=self.repair_fixture()
        for data in (b'x'*8193,b'{"schema":1,"schema":1}',json.dumps(dict(value,extra=float('inf'))).encode(),
                     json.dumps(value).replace('"schema": 1','"schema": 1e999').encode()):
            path.write_bytes(data);receipt={};r.observe_float_repair(home,receipt)
            self.assertEqual(receipt['float_range_repair']['status'],'rejected')
        path.unlink();other=self.root/'other.json';other.write_text(json.dumps(value),encoding='utf-8');path.symlink_to(other)
        receipt={};r.observe_float_repair(home,receipt);self.assertEqual(receipt['float_range_repair']['status'],'rejected')
        path.unlink();path.write_text(json.dumps(value),encoding='utf-8')
        (home/'builds-latest-gnu'/('b'*64)).mkdir();receipt={};r.observe_float_repair(home,receipt)
        self.assertEqual(receipt['float_range_repair']['status'],'rejected')

    def test_same_build_attempt_finally_collects_without_a_second_child(self):
        tree=ast.parse((r.HERE/'run.py').read_bytes());execute=next(n for n in tree.body if isinstance(n,ast.FunctionDef) and n.name=='execute')
        tries=[n for n in ast.walk(execute) if isinstance(n,ast.Try) and any(isinstance(x,ast.Expr) and isinstance(x.value,ast.Call) and isinstance(x.value.func,ast.Name) and x.value.func.id=='observe_float_repair' for x in n.finalbody)]
        self.assertEqual(len(tries),1);node=tries[0];events=[];receipt={}
        def phase(*a,**kw):events.append('same build');raise RuntimeError('original build')
        def observe(*a):events.append('observe');r.observe_float_repair(self.root/'absent',receipt)
        scope={'phase':phase,'observe_float_repair':observe,'sys':r.sys,'ordinary':self.root,
               'horizon':self.root,'mixins':self.root,'toolchain':self.root,'work':self.root,'env':{},'receipt':receipt}
        with self.assertRaisesRegex(RuntimeError,'original build'):
            exec(compile(ast.Module(body=[node],type_ignores=[]),'<actual-build-try>','exec'),scope)
        self.assertEqual(events,['same build','observe']);self.assertEqual(receipt['float_range_repair']['status'],'absent')
        text=ast.unparse(execute)
        self.assertLess(text.index("['status'] != 'observed'"),text.index('linked_result('))

    def syntax_fixture(self):
        home,stage,path,repair=self.repair_fixture();formatter=self.root/'selected-rustfmt.exe'
        value={'schema':1,'kind':'rustfmt-stdin-syntax','passed':True,'syntax_only':True,'type_checked':False,
          'target':repair['target'],'input_sha256':repair['after_sha256'],'input_bytes':repair['after_bytes'],
          'edition':'2024','stdin_nonrecursive':True,'emit':'stdout','writes_source':False,'timeout_seconds':30,
          'formatter':str(formatter),'formatter_sha256':'d'*64,'returncode':0,'stderr_bytes':0,
          'source_unchanged':True,'formatter_unchanged':True}
        syntax=stage/'rust-syntax-receipt.json';syntax.write_text(json.dumps(value),encoding='utf-8')
        return home,stage,syntax,value,formatter

    def test_actual_syntax_receipt_links_same_repaired_bytes_and_selected_formatter(self):
        home,stage,path,value,formatter=self.syntax_fixture();receipt={'gates':{'formatter':{'executable_sha256':'d'*64}}}
        r.observe_float_repair(home,receipt,formatter)
        self.assertEqual(receipt['float_range_repair']['status'],'observed')
        self.assertEqual(receipt['rust_syntax']['status'],'observed');self.assertTrue(receipt['rust_syntax']['result']['passed'])
        failed=dict(value,passed=False,returncode=1,stderr_bytes=100,error='ValueError: syntax rejected')
        path.write_text(json.dumps(failed),encoding='utf-8');receipt['error']='original failed phase'
        r.observe_float_repair(home,receipt,formatter)
        self.assertEqual(receipt['rust_syntax']['status'],'observed');self.assertFalse(receipt['rust_syntax']['result']['passed'])
        self.assertEqual(receipt['error'],'original failed phase')

    def test_syntax_false_success_or_changed_input_formatter_is_rejected(self):
        home,stage,path,value,formatter=self.syntax_fixture()
        for key,bad in [('passed',1),('input_sha256','0'*64),('formatter_sha256','0'*64),('formatter','elsewhere'),
                        ('returncode',True),('returncode',1),('stderr_bytes',False),('source_unchanged',False),('error','unexpected')]:
            path.write_text(json.dumps(dict(value,**{key:bad})),encoding='utf-8');receipt={'gates':{'formatter':{'executable_sha256':'d'*64}}}
            r.observe_float_repair(home,receipt,formatter);self.assertEqual(receipt['rust_syntax']['status'],'rejected')
            self.assertEqual(receipt['float_range_repair']['status'],'observed')
        path.write_bytes(b'x'*16385);receipt={'gates':{'formatter':{'executable_sha256':'d'*64}}}
        r.observe_float_repair(home,receipt,formatter);self.assertEqual(receipt['rust_syntax']['status'],'rejected')

    def test_syntax_success_is_required_before_link_evidence(self):
        source=(r.HERE/'run.py').read_text(encoding='utf-8')
        self.assertIn("observe_float_repair(work/'home',receipt,toolchain/'bin/rustfmt.exe')",source)
        self.assertLess(source.index("raise ValueError('Successful link lacks accepted syntax evidence')"),source.index("linked,detail=linked_result(raw"))
        self.assertEqual(source.count("phase('build-only'"),1)

if __name__=='__main__':unittest.main()
