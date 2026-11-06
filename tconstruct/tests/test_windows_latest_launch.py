"""Pure admission/source tests. No PowerShell, Git, Cargo, Windows or game execution."""
import ast
import hashlib
import importlib.util
import json
from pathlib import Path
import struct
import sys
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0,str(ROOT/'tools'))
import play_windows_latest as m

class Tests(unittest.TestCase):
    def setUp(self):
        self.tmp=tempfile.TemporaryDirectory(); self.addCleanup(self.tmp.cleanup)
        self.root=Path(self.tmp.name)
    def workspace(self,mods=False):
        root=self.root/('with' if mods else 'without'); (root/'crates/horizon-client').mkdir(parents=True)
        (root/'crates/horizon-client/Cargo.toml').write_text('[features]\ndefault=["audio","controllers"]\n'+('mods=[]\n' if mods else ''))
        return root
    def test_official_pins_and_no_legacy_compat(self):
        value={'horizon_repository':'https://github.com/horizon-rust/horizon.git','mixins_repository':'https://github.com/horizon-rust/horizon-mixins.git','horizon_revision':m.HORIZON,'mixins_revision':m.MIXINS}
        m.validate_compat(value)
        for key in value:
            with self.assertRaises(ValueError): m.validate_compat({**value,key:'wrong'})
    def test_rust_host_and_release_exact(self):
        m.validate_rust('rustc 1.99.1 (fixed)\nrelease: 1.99.1\nhost: x86_64-pc-windows-gnu\n')
        for text in ('release: 1.99.1\nhost: x86_64-pc-windows-msvc','release: 1.95.0\nhost: '+m.TARGET,'release: 1.99.1'):
            with self.assertRaises(ValueError):m.validate_rust(text)
    def test_default_gameplay_without_nonexistent_mods(self):
        stage=self.workspace(); a=m.compile_args(Path('cargo.exe'),stage,Path('target'))
        self.assertNotIn('--no-default-features',a); self.assertNotIn('--features',a)
        for pair in [('--locked','--offline'),('--profile','fast'),('--target',m.TARGET)]:
            self.assertTrue(any(a[i:i+2]==list(pair) for i in range(len(a)-1)))
    def test_declared_mods_and_exact_config(self):
        stage=self.workspace(True); (stage/'.cargo').mkdir(); (stage/'.cargo/config.toml').write_text('')
        a=m.compile_args(Path('cargo.exe'),stage,Path('target'))
        self.assertEqual(a[a.index('--features')+1],m.MODS_FEATURE)
        self.assertEqual(a[a.index('--config')+1],str(stage/'.cargo/config.toml'))
    def test_unexpected_default_features_rejected(self):
        stage=self.workspace(); p=stage/'crates/horizon-client/Cargo.toml'; p.write_text('[features]\ndefault=[]\n')
        with self.assertRaises(ValueError):m.feature_contract(stage)
    def test_environment_scrub_and_cli_selected_toolchain(self):
        env=m.build_environment({'PATH':'original','RUSTFLAGS':'bad','RUSTUP_TOOLCHAIN':'old','RUSTC_WRAPPER':'wrap','CC':'bad','CARGO_BUILD_TARGET':'wrong','CARGO_PROFILE_FAST_OPT_LEVEL':'0'},Path('tool'),Path('home'))
        for key in ('RUSTFLAGS','RUSTUP_TOOLCHAIN','RUSTC_WRAPPER','CARGO_BUILD_TARGET','CARGO_PROFILE_FAST_OPT_LEVEL'):self.assertNotIn(key,env)
        self.assertEqual(env['RUSTC'],str(Path('tool/bin/rustc.exe')))
        self.assertEqual(env['CARGO_NET_OFFLINE'],'true')
        self.assertTrue(env['PATH'].startswith(str(Path('tool/bin'))))
        self.assertEqual(env['CC'],str(Path('tool/mingw64/bin/gcc.exe')))
    def test_input_tree_content_names_and_mtime(self):
        r=self.root/'r';r.mkdir();p=r/'a';p.write_bytes(b'a');a=m.tree_digest(r)
        p.touch();self.assertEqual(a,m.tree_digest(r));p.rename(r/'b');self.assertNotEqual(a,m.tree_digest(r))
        a=m.tree_digest(r);(r/'b').write_bytes(b'b');self.assertNotEqual(a,m.tree_digest(r))
    def test_link_parent_and_special_inputs_rejected(self):
        r=self.root/'r';r.mkdir();(r/'a').write_text('a');(r/'link').symlink_to(r/'a')
        with self.assertRaises(ValueError):m.tree_digest(r)
        (r/'link').unlink(); alias=self.root/'alias';alias.symlink_to(r,target_is_directory=True)
        with self.assertRaises(ValueError):m.tree_digest(alias)
    def test_resource_budget_rejects(self):
        r=self.root/'r';r.mkdir();(r/'a').write_bytes(b'a')
        for field in ('MAX_FILES','MAX_TOTAL'):
            with patch.object(m,field,0):
                with self.assertRaises(ValueError):m.tree_digest(r)
    def test_disjoint_inputs_output(self):
        m.disjoint([('a',self.root/'a'),('b',self.root/'b')])
        for other in (self.root/'a',self.root/'a/b',self.root):
            with self.assertRaises(ValueError):m.disjoint([('a',self.root/'a'),('b',other)])
    def test_parent_cargo_config_rejected(self):
        (self.root/'.cargo').mkdir();(self.root/'.cargo/config.toml').write_text('')
        with self.assertRaises(ValueError):m.no_parent_configs(self.root/'child')
    def test_cache_identity_all_dimensions(self):
        original={'source':'a','lock':'b','toolchain':'c','target':m.TARGET,'features':['audio','controllers'],'pluto':'d','resources':'e'}
        key=m.ready_identity(original)
        for name in original:self.assertNotEqual(key,m.ready_identity({**original,name:'changed'}))
    def test_windows_pe_architecture_check(self):
        p=self.root/'x.exe';raw=bytearray(80);raw[:2]=b'MZ';raw[60:64]=struct.pack('<I',64);raw[64:70]=b'PE\0\0\x64\x86';p.write_bytes(raw);m.pe_x64(p)
        raw[68:70]=b'\x4c\x01';p.write_bytes(raw)
        with self.assertRaises(ValueError):m.pe_x64(p)
    def test_stage_exact_two_edit_inverse_and_guard_order(self):
        x=json.loads((ROOT/'tests/windows-latest-stage-inverse.json').read_text());s=(ROOT/'tools/stage.py').read_text()
        self.assertEqual(hashlib.sha256(s.encode()).hexdigest(),m.STAGE_SHA)
        self.assertEqual(m.STAGE_SHA,x['candidate_sha256'])
        self.assertLess(s.index("'plan', '--workspace'"),s.index('pluto-lib/tools/check_native_plan.py'))
        self.assertLess(s.index('pluto-lib/tools/check_native_plan.py'),s.index("'apply', '--workspace'"))
        for old,new in reversed(x['edits']):self.assertEqual(s.count(new),1);s=s.replace(new,old,1)
        self.assertEqual(hashlib.sha256(s.encode()).hexdigest(),x['base_sha256'])
    def test_ps_is_connected_and_has_no_discovery_install(self):
        s=(ROOT/'tools/play-windows.ps1').read_text()
        self.assertIn('play_windows_latest.py',s)
        for token in ('Get-ChildItem','A:\\Project\\horizon','Checkout-Pinned','git clone','rustup install','--no-default-features'):self.assertNotIn(token,s)
        for name in ('horizon','mixins','pluto-bundle','resources','toolchain','home'):self.assertIn('--'+name,s)
    def test_launch_order_no_fallback_or_automatic_fetch(self):
        s=(ROOT/'tools/play_windows_latest.py').read_text();ast.parse(s)
        self.assertLess(s.index("repo/'tools/stage.py','--horizon'"),s.index('run(compile_args('))
        self.assertLess(s.index('if snapshot()!=initial'),s.index('ready.write_text'))
        self.assertLess(s.index("receipt.get('outputs')!=outputs()"),s.index("env['HORIZON_HOME']"))
        for token in ("'clone'","'fetch'","'install'",'rmtree','unlink('):self.assertNotIn(token,s)
        self.assertIn("game-latest-",s)

if __name__=='__main__': unittest.main()
