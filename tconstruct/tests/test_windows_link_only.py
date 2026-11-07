"""Controller-only link mode tests. All compiler/guard/game commands are mocked."""
import ast
from contextlib import ExitStack, redirect_stdout
import hashlib
import io
import json
import os
from pathlib import Path
import struct
import subprocess
import sys
import tempfile
from types import SimpleNamespace
import unittest
from windows_source_layers import before_package_contract
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / 'tools'))
import play_windows_latest as m


class FakePreparedWindows:
    """Prepared inputs and command log, not an emulation of the Windows API."""
    def __init__(self, root, mode=True, fail=None, drift=False):
        self.root=root; self.calls=[]; self.fail=fail; self.drift=drift; self.compiled=False
        for name in ('horizon','mixins','toolchain','resources'):
            (root/name).mkdir()
        for name in ('bin/rustc.exe','bin/cargo.exe','bin/rustdoc.exe','bin/rustfmt.exe',
                     'mingw64/bin/gcc.exe','mingw64/bin/g++.exe','mingw64/bin/ar.exe',
                     'mingw64/bin/ranlib.exe','mingw64/bin/windres.exe'):
            p=root/'toolchain'/name; p.parent.mkdir(exist_ok=True,parents=True); p.write_bytes(b'mocked-tool')
        (root/'resources/horizon-resources.json').write_bytes(b'{}')
        archive=root/'formatter.tar.gz'; archive.write_bytes(b'mocked-archive')
        self.args=SimpleNamespace(build_only=mode,resources=None if mode else root/'resources',
            horizon=root/'horizon',mixins=root/'mixins',toolchain=root/'toolchain',home=root/'home',
            rustfmt_archive=archive,pluto_bundle=None)

    def run(self, argv, **kwargs):
        argv=list(map(str,argv)); self.calls.append(argv)
        env=kwargs['env']
        if env.get('PYTHONUTF8')!='1' or env.get('PYTHONIOENCODING')!='utf-8':
            raise AssertionError('Selected child encoding lost')
        if argv[0].endswith('rustfmt.exe'): return 'rustfmt 1.10.0-stable (58cf8f9c35 2026-10-12)'
        if argv[0].endswith('rustc.exe'): return 'release: 1.99.1\nhost: x86_64-pc-windows-gnu'
        if argv[0].endswith('cargo.exe') and argv[1]=='-vV': return 'cargo 1.99.1 (mocked)'
        if argv[0].endswith('gcc.exe'): return '16.2.0' if argv[1]=='-dumpfullversion' else 'x86_64-w64-mingw32'
        if argv[1].endswith('/tools/stage.py'):
            if self.fail=='guard': raise subprocess.CalledProcessError(1,argv)
            return None
        if argv[0].endswith('cargo.exe') and argv[1]=='build':
            if self.fail=='link': raise subprocess.CalledProcessError(1,argv)
            p=self.args.home/'cargo-target-latest-gnu'/m.TARGET/m.PROFILE/'horizon-client.exe'
            p.parent.mkdir(parents=True,exist_ok=True)
            raw=bytearray(80); raw[:2]=b'MZ'; raw[60:64]=struct.pack('<I',64);raw[64:70]=b'PE\0\0\x64\x86'
            p.write_bytes(raw);self.compiled=True; return None
        if self.args.build_only: raise AssertionError('Link-only reached an asset/game command: '+repr(argv))
        if '--tconstruct-prepare' in argv: Path(argv[-1]).mkdir(parents=True)
        return None

    def execute(self):
        real_read=m.read
        def read(path):
            if path==ROOT.parent/'pluto-lib/horizon-native.json':return b'{}'
            return real_read(path)
        native_os=SimpleNamespace(name='nt',environ=dict(os.environ),path=os.path,devnull=os.devnull,pathsep=os.pathsep)
        with ExitStack() as stack:
            for name,value in [('os',native_os),('read',read),('run',self.run),
                               ('no_parent_configs',lambda p:None),('tree_digest',lambda *a,**k:'fixed-tree'),
                               ('source_identity',lambda *a,**k:{'fixed':not(self.drift and self.compiled)}),
                               ('feature_contract',lambda p:{'defaults':['audio','controllers'],'extra':[]}),
                               ('fingerprint',lambda *a:'fixed-bundle')]:
                stack.enter_context(patch.object(m,name,value))
            stack.enter_context(patch.object(m.platform,'machine',return_value='AMD64'))
            stack.enter_context(patch('shutil.which',return_value='/mocked/git'))
            stack.enter_context(patch.object(m.guard_format,'require_guard_routing',return_value=None))
            stack.enter_context(patch.object(m.guard_format,'admit',return_value={'mocked':'formatter'}))
            stack.enter_context(patch.object(m.bundles,'prepare',return_value=(self.root/'mods',self.root/'bundle',{'packages':[{'archive_sha256':'a'*64},{'archive_sha256':'b'*64}]})))
            stack.enter_context(patch.object(m.bundles,'verify_prepared',return_value=None))
            output=io.StringIO()
            with redirect_stdout(output):m.execute(self.args)
            return output.getvalue()


class Tests(unittest.TestCase):
    def setUp(self):
        self.tmp=tempfile.TemporaryDirectory();self.addCleanup(self.tmp.cleanup);self.root=Path(self.tmp.name)

    def test_modes_require_resources_only_for_game_and_reject_ambiguous_input(self):
        self.assertEqual(m.link_mode(True,None),'link-only')
        self.assertEqual(m.link_mode(False,Path('resources')),'playtest')
        with self.assertRaises(ValueError):m.link_mode(True,Path('resources'))
        with self.assertRaises(ValueError):m.link_mode(False,None)

    def test_exact_inverse_preserves_prior_runtime_outside_mode_changes(self):
        spec=json.loads((ROOT/'tests/windows-link-only-inverse.json').read_text(encoding='utf-8'))
        for relative,row in spec['files'].items():
            raw=before_package_contract(relative,(ROOT/relative).read_bytes(),before_stat=True);self.assertEqual(hashlib.sha256(raw).hexdigest(),row['after_sha256'])
            text=raw.decode('utf-8')
            for old,new in reversed(row['edits']):
                self.assertEqual(text.count(new),1);text=text.replace(new,old,1)
            self.assertEqual(hashlib.sha256(text.encode()).hexdigest(),row['before_sha256'])

    def test_mocked_link_has_guarded_stage_then_default_gnu_build_no_assets_or_game(self):
        fake=FakePreparedWindows(self.root);text=fake.execute();result=json.loads(text)
        stage=next(i for i,a in enumerate(fake.calls) if len(a)>1 and a[1].endswith('/tools/stage.py'))
        build=next(i for i,a in enumerate(fake.calls) if len(a)>1 and a[1]=='build')
        self.assertLess(stage,build);args=fake.calls[build]
        for flag,value in [('--target',m.TARGET),('--profile','fast'),('-p','horizon-client'),('--bin','horizon-client')]:
            self.assertEqual(args[args.index(flag)+1],value)
        self.assertIn('--locked',args);self.assertIn('--offline',args);self.assertNotIn('--no-default-features',args)
        self.assertFalse(result['game_started']);self.assertFalse(result['resources_prepared'])
        self.assertTrue(result['receipt'].endswith('/.linked.json'))
        self.assertEqual(list(fake.args.home.rglob('.ready.json')),[])
        self.assertEqual(list(fake.args.home.glob('game-*')),[])
        receipt=json.loads(Path(result['receipt']).read_text(encoding='utf-8'))
        self.assertEqual(receipt['inputs']['mode'],'link-only');self.assertIsNone(receipt['inputs']['resources'])
        self.assertEqual(set(receipt['outputs']),{'game'})

    def test_mocked_guard_failure_never_reaches_link_or_receipt(self):
        fake=FakePreparedWindows(self.root,fail='guard')
        with self.assertRaises(subprocess.CalledProcessError):fake.execute()
        self.assertFalse(any(len(a)>1 and a[1]=='build' for a in fake.calls))
        self.assertEqual(list(fake.args.home.rglob('.linked.json')),[])

    def test_mocked_link_failure_never_writes_receipt(self):
        fake=FakePreparedWindows(self.root,fail='link')
        with self.assertRaises(subprocess.CalledProcessError):fake.execute()
        self.assertEqual(list(fake.args.home.rglob('.linked.json')),[])

    def test_mocked_changed_source_after_link_rejects_receipt(self):
        fake=FakePreparedWindows(self.root,drift=True)
        with self.assertRaisesRegex(ValueError,'Inputs changed'):fake.execute()
        self.assertEqual(list(fake.args.home.rglob('.linked.json')),[])

    def test_link_cache_rechecks_binary_and_never_becomes_playtest_ready(self):
        fake=FakePreparedWindows(self.root);result=json.loads(fake.execute());fake.calls.clear()
        again=json.loads(fake.execute());self.assertEqual(result,again)
        self.assertFalse(any(len(a)>1 and a[1]=='build' for a in fake.calls))
        Path(result['executable']).write_bytes(b'changed')
        with self.assertRaisesRegex(ValueError,'Cached input/output identity mismatch'):fake.execute()
        self.assertNotEqual(m.ready_identity({'mode':'link-only','resources':None}),
                            m.ready_identity({'mode':'playtest','resources':'prepared'}))

    def test_mocked_normal_mode_retains_preparation_and_game_startup(self):
        fake=FakePreparedWindows(self.root,mode=False);fake.execute()
        self.assertTrue(any('--tconstruct-prepare' in a for a in fake.calls))
        self.assertEqual(fake.calls[-1][1],'--resources')
        self.assertEqual(len(list(fake.args.home.rglob('.ready.json'))),1)
        self.assertEqual(list(fake.args.home.rglob('.linked.json')),[])

    def test_parser_and_powershell_route_explicit_build_only_without_asset_selector(self):
        text=(ROOT/'tools/play_windows_latest.py').read_text(encoding='utf-8');ast.parse(text)
        self.assertIn("p.add_argument('--build-only',action='store_true'",text)
        self.assertIn("p.add_argument('--resources',type=Path",text)
        ps=(ROOT/'tools/play-windows.ps1').read_text(encoding='utf-8')
        self.assertIn('[switch]$BuildOnly',ps);self.assertIn("@('--build-only')",ps)
        self.assertIn('@ResourceArguments',ps)
        selected=ps.split('if ($BuildOnly) {',1)[1].split('} else {',1)[0]
        self.assertNotIn('Pick-Folder',selected)
        self.assertIn("if ($Resources) { throw",selected)


if __name__=='__main__':unittest.main()
