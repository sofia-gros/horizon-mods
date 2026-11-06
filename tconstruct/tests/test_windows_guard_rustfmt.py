"""Pure formatter admission/routing tests. No formatter, compiler or Windows execution."""
import ast
import hashlib
import importlib.util
import io
import os
from pathlib import Path
import sys
import tarfile
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT/'tools'))
import playtest_guard_rustfmt as f
import play_windows_latest as launch


def load(name):
    spec = importlib.util.spec_from_file_location(name, ROOT.parent/'pluto-lib/tools'/f'{name}.py')
    module = importlib.util.module_from_spec(spec); spec.loader.exec_module(module); return module


class Tests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory(); self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)

    def archive(self, members=None):
        stream = io.BytesIO()
        with tarfile.open(fileobj=stream, mode='w:gz') as archive:
            for name, raw, kind in members or [(f.MEMBER, b'fixture-executable', tarfile.REGTYPE)]:
                info = tarfile.TarInfo(name); info.size = len(raw); info.type = kind
                if kind == tarfile.SYMTYPE: info.linkname = 'other'
                archive.addfile(info, io.BytesIO(raw) if kind == tarfile.REGTYPE else None)
        return stream.getvalue()

    def test_official_manifest_and_sidecar_pins(self):
        self.assertEqual(f.ARCHIVE_SHA256, '488f4cdcec501721fb2986a4c36c8180d5606f5b569f942fb7c8886f36a7e2aa')
        self.assertEqual((f.VERSION, f.COMMIT), ('1.10.0', '58cf8f9c35b355199727627d07ae34a4e2846d67'))
        self.assertEqual(f.URL, 'https://static.rust-lang.org/dist/2026-10-15/rustfmt-1.99.1-x86_64-pc-windows-gnu.tar.gz')

    def test_launcher_rejects_missing_or_old_guard_routing_before_execution(self):
        f.require_guard_routing(ROOT.parent/'pluto-lib')
        with self.assertRaises(FileNotFoundError): f.require_guard_routing(self.root)
        (self.root/'tools').mkdir()
        for name in f.GUARD_ROUTING: (self.root/'tools'/name).write_bytes(b'old guard with ambient rustfmt')
        with self.assertRaisesRegex(ValueError, 'routing'): f.require_guard_routing(self.root)
        source=(ROOT/'tools/play_windows_latest.py').read_text(encoding='utf-8')
        self.assertLess(source.index('guard_format.require_guard_routing(pluto)'), source.index('formatter_version = run('))

    def test_unpinned_archive_rejects_before_parsing(self):
        with self.assertRaisesRegex(ValueError, 'exact official'): f.archive_binary(b'not an archive')

    def test_synthetic_archive_matches_exact_selected_executable(self):
        raw = self.archive(); tool = self.root/'tool'; (tool/'bin').mkdir(parents=True)
        (tool/'bin/rustfmt.exe').write_bytes(b'fixture-executable'); archive = self.root/'rustfmt.tar.gz'; archive.write_bytes(raw)
        with patch.object(f, 'ARCHIVE_SHA256', f.sha(raw)):
            result = f.admit(tool, archive)
            self.assertEqual(result['executable_sha256'], f.sha(b'fixture-executable'))
            (tool/'bin/rustfmt.exe').write_bytes(b'foreign-executable')
            with self.assertRaisesRegex(ValueError, 'differs'): f.admit(tool, archive)

    def test_missing_member_duplicates_and_links_reject(self):
        for rows in [[('other', b'x', tarfile.REGTYPE)],
                     [(f.MEMBER, b'x', tarfile.REGTYPE), (f.MEMBER, b'x', tarfile.REGTYPE)],
                     [(f.MEMBER, b'', tarfile.SYMTYPE)],
                     [('../escape', b'x', tarfile.REGTYPE)]]:
            raw = self.archive(rows)
            with patch.object(f, 'ARCHIVE_SHA256', f.sha(raw)):
                with self.assertRaises(ValueError): f.archive_binary(raw)

    def test_member_expansion_and_input_budgets_reject(self):
        raw = self.archive()
        with patch.object(f, 'ARCHIVE_SHA256', f.sha(raw)):
            for name in ('MEMBER_LIMIT', 'EXPANDED_LIMIT', 'ARCHIVE_LIMIT'):
                with patch.object(f, name, 0):
                    with self.assertRaises(ValueError): f.archive_binary(raw)
        path = self.root/'input'; path.write_bytes(b'12')
        with self.assertRaises(ValueError): f.bounded_file(path, 1)

    def test_archive_and_executable_parent_links_reject(self):
        real = self.root/'real'; real.mkdir(); (real/'input').write_bytes(b'x')
        alias = self.root/'alias'; alias.symlink_to(real, target_is_directory=True)
        with self.assertRaises(ValueError): f.bounded_file(alias/'input', 10)

    def test_version_package_commit_and_channel_are_exact(self):
        f.validate_version('rustfmt 1.10.0-stable (58cf8f9c35 2026-10-12)')
        for text in ['rustfmt 1.8.0-stable (58cf8f9c35 2026-10-12)',
                     'rustfmt 1.10.0-nightly (58cf8f9c35 2026-10-12)',
                     'rustfmt 1.10.0-stable (0000000000 2026-10-12)',
                     'rustfmt 1.10.0', 'rustfmt 1.10.0-stable (58cf8f9c35 2026-10-12) extra']:
            with self.assertRaises(ValueError): f.validate_version(text)

    def test_launcher_absolute_formatter_scrubs_inherited_selection(self):
        tool = self.root/'tool'; env = launch.build_environment({'PATH':'foreign', 'PLUTO_GUARD_RUSTFMT':'foreign.exe'}, tool, self.root/'home')
        self.assertEqual(env['PLUTO_GUARD_RUSTFMT'], str(tool/'bin/rustfmt.exe'))
        self.assertTrue(Path(env['PLUTO_GUARD_RUSTFMT']).is_absolute())
        self.assertEqual(env['CARGO'], str(tool/'bin/cargo.exe'))

    def test_cell_guard_routes_only_explicit_absolute_tool_or_legacy_default(self):
        guard = load('check_terrain_cell_plan'); calls = []
        def run(argv, **kwargs): calls.append(argv); return SimpleNamespace(stdout='unchanged')
        with patch.object(guard.subprocess, 'run', run), patch.dict(os.environ, {}, clear=True):
            self.assertEqual(guard.formatted('body'), 'unchanged'); self.assertEqual(calls[-1][0], 'rustfmt')
            os.environ['PLUTO_GUARD_RUSTFMT'] = str(self.root/'rustfmt.exe')
            guard.formatted('body'); self.assertEqual(calls[-1], [str(self.root/'rustfmt.exe'), '--edition', '2024', '--emit', 'stdout'])
            for bad in ('', 'rustfmt', './rustfmt'):
                os.environ['PLUTO_GUARD_RUSTFMT'] = bad
                with self.assertRaises(ValueError): guard.formatted('body')

    def test_damage_guard_preserves_hunks_and_exact_result_with_explicit_tool(self):
        guard = load('check_damage_equipment_plan'); source = 'fn native() {}\nEND'
        (self.root/'native.rs').write_text(source); target = self.root/'mixins/pluto_native_creative'; target.mkdir(parents=True)
        expected = 'fn original() {}'; (target/'helper.rs').write_text(expected)
        meta = {'file':'native.rs','file_sha256':guard.digest(source),'start':'fn native(', 'end':'\nEND',
                'function_sha256':guard.digest('fn native() {}'),'declaration':'fn native(', 'changes':[],
                'header':'','footer':'','helper':'helper.rs','injection_id':'test'}
        plan = {'injections':[{'mod_id':'pluto_native_creative','injection_id':'test','helper_source':expected}]}; calls = []
        def run(argv, **kwargs): calls.append(argv); self.assertEqual(kwargs['input'], expected); return SimpleNamespace(stdout=expected)
        with patch.object(guard, 'ROOT', self.root), patch.object(guard.subprocess, 'run', run), patch.dict(os.environ, {'PLUTO_GUARD_RUSTFMT':str(self.root/'rustfmt.exe')}, clear=True):
            guard.verify_source(plan,self.root,[meta]); self.assertEqual(calls[-1][0],str(self.root/'rustfmt.exe'))
            os.environ['PLUTO_GUARD_RUSTFMT'] = 'relative.exe'
            with self.assertRaises(ValueError): guard.verify_source(plan,self.root,[meta])
            os.environ['PLUTO_GUARD_RUSTFMT'] = str(self.root/'rustfmt.exe')
            plan['injections'][0]['helper_source'] = 'changed'
            with self.assertRaises(ValueError): guard.verify_source(plan,self.root,[meta])

    def test_admission_precedes_execution_and_enters_ready_identity(self):
        source = (ROOT/'tools/play_windows_latest.py').read_text(encoding='utf-8'); ast.parse(source)
        self.assertLess(source.index('toolchain_before = tree_digest'),source.index('formatter = guard_format.admit'))
        self.assertLess(source.index('formatter = guard_format.admit'),source.index("formatter_version = run([toolchain/'bin/rustfmt.exe'"))
        self.assertIn("'formatter':{**guard_format.admit(toolchain,rustfmt_archive),'version':formatter_version}",source)
        self.assertIn("p.add_argument('--rustfmt-archive',type=Path,required=True",source)
        ps = (ROOT/'tools/play-windows.ps1').read_text(encoding='utf-8')
        self.assertIn('--rustfmt-archive $RustfmtArchive', ps)
        self.assertNotIn('Get-ChildItem',ps)

    def test_guard_inverse_changes_only_formatter_routing(self):
        facts = {
            'check_terrain_cell_plan.py': ('49a7ac329087f8d4e4cf0df7927dd16afad4f668678fb1603be188b747a97ed6', 4),
            'check_damage_equipment_plan.py': ('53e54d594c815e6aba623f47f627138e5545c095296a92f1977071d54ec48fac', 8),
        }
        for name, (expected, indent) in facts.items():
            source = (ROOT.parent/'pluto-lib/tools'/name).read_text(encoding='utf-8')
            self.assertEqual(source.count('import os\n'),1); source=source.replace('import os\n','',1)
            space=' '*indent
            extra=(space+"formatter = os.environ.get('PLUTO_GUARD_RUSTFMT', 'rustfmt')\n"+
                   space+"if 'PLUTO_GUARD_RUSTFMT' in os.environ and not Path(formatter).is_absolute():\n"+
                   space+"    raise ValueError('Explicit guard formatter must be an absolute path')\n")
            self.assertEqual(source.count(extra),1);source=source.replace(extra,'',1)
            if indent==4: source=source.replace('[formatter,',"['rustfmt',",1)
            else: source=source.replace('[formatter,','["rustfmt",',1)
            self.assertEqual(hashlib.sha256(source.encode()).hexdigest(),expected)


if __name__ == '__main__': unittest.main()
