"""Pure file/hash/source regressions, not PowerShell or Windows/native execution."""
import hashlib
import importlib.util
import json
from pathlib import Path
import stat
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location('inputs', ROOT/'tools/playtest_pluto_inputs.py')
m = importlib.util.module_from_spec(spec); spec.loader.exec_module(m)

def legacy_script():
    """Verify exact latest entry, then test the preserved historical 60e42 source."""
    text=(ROOT/'tools/play-windows.ps1').read_text(encoding='utf-8')
    latest=ROOT/'tests/windows-latest-entry-inverse.json'
    if latest.exists():
        value=json.loads(latest.read_text(encoding='utf-8'))
        if hashlib.sha256(text.encode()).hexdigest()!=value['candidate_script_sha256']:
            raise ValueError('Latest entry differs from frozen inverse witness')
        text=value['base_script']
        if hashlib.sha256(text.encode()).hexdigest()!=value['base_script_sha256']:
            raise ValueError('Historical entry inverse drift')
    return text

class Tests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(); self.addCleanup(self.temp.cleanup)
        root = Path(self.temp.name); self.bundle = root/'bundle'; self.pluto = root/'pluto'
        for path in [self.bundle, self.pluto/'tools', self.pluto/'pluto_lib', self.pluto/'mixins/pluto_native_creative']:
            path.mkdir(parents=True, exist_ok=True); (path/'source.rs').write_bytes(b'fn initial() {}\n')
        (self.bundle/'horizon-mixin.toml').write_text('[mod]\nid="pluto_native_creative"\n')
    def digest(self): return m.fingerprint(self.bundle, self.pluto)
    def test_repeat_and_same_bytes_new_mtime(self):
        before=self.digest(); (self.bundle/'source.rs').touch(); self.assertEqual(self.digest(),before)
        self.assertRegex(before,r'^[0-9a-f]{64}$')
    def test_bundle_source_change_invalidates(self):
        before=self.digest(); (self.bundle/'source.rs').write_bytes(b'fn changed() {}\n'); self.assertNotEqual(before,self.digest())
    def test_guard_tools_and_reviewed_sources_invalidate(self):
        for part in ['tools','pluto_lib','mixins/pluto_native_creative']:
            before=self.digest(); (self.pluto/part/'source.rs').write_bytes(b'changed'); self.assertNotEqual(before,self.digest())
    def test_file_add_delete_and_rename_invalidate(self):
        before=self.digest(); path=self.bundle/'new.bin'; path.write_bytes(b'\0\xff'); added=self.digest(); self.assertNotEqual(before,added)
        path.rename(self.bundle/'renamed.bin'); self.assertNotEqual(added,self.digest())
        (self.bundle/'renamed.bin').unlink(); self.assertEqual(before,self.digest())
    def test_interpreter_cache_only_is_ignored(self):
        before=self.digest(); (self.pluto/'tools/__pycache__').mkdir(); (self.pluto/'tools/__pycache__/code.pyc').write_bytes(b'cache')
        (self.pluto/'tools/code.pyc').write_bytes(b'cache'); self.assertEqual(before,self.digest())
    def test_missing_root_or_manifest_rejects(self):
        (self.bundle/'horizon-mixin.toml').unlink()
        with self.assertRaises(ValueError):self.digest()
        with self.assertRaises(OSError):m.fingerprint(self.bundle/'missing',self.pluto)
    def test_file_symlink_and_parent_symlink_reject(self):
        (self.bundle/'link.rs').symlink_to(self.pluto/'tools/source.rs')
        with self.assertRaises(ValueError):self.digest()
        (self.bundle/'link.rs').unlink(); alias=Path(self.temp.name)/'alias'; alias.symlink_to(self.bundle,target_is_directory=True)
        with self.assertRaises(ValueError):m.fingerprint(alias,self.pluto)
    def test_windows_reparse_and_special_modes_reject(self):
        for mode,attributes in [(stat.S_IFDIR|0o755,m.REPARSE),(stat.S_IFREG|0o644,m.REPARSE),(stat.S_IFIFO|0o600,0)]:
            with self.assertRaises(ValueError):m.plain(SimpleNamespace(st_mode=mode,st_file_attributes=attributes))
    def test_budgets_fail_closed(self):
        for limit in ['MAX_FILES','MAX_FILE_BYTES','MAX_TOTAL_BYTES']:
            with patch.object(m,limit,0):
                with self.assertRaises(ValueError):self.digest()
    def test_changed_file_stat_rejects(self):
        path=self.bundle/'source.rs'; before=path.stat(); path.write_bytes(b'changed length')
        with self.assertRaisesRegex(ValueError,'changed before read'):m.file_hash(path,before)
    def test_script_exact_inverse_keeps_other_behavior(self):
        value=json.loads((ROOT/'tests/windows-pluto-cache-inverse.json').read_text(encoding='utf-8')); text=legacy_script()
        self.assertEqual(hashlib.sha256(text.encode()).hexdigest(),value['candidate_script_sha256'])
        for old,new in reversed(value['edits']):
            if not new:
                # Removed inner prompt has its fixed insertion anchor retained.
                anchor="        try {\n            Run py @('-3', 'tools/stage.py'"
                self.assertEqual(text.count(anchor),1)
                text=text.replace(anchor,"        try {\n"+old+"            Run py @('-3', 'tools/stage.py'",1)
            else:
                self.assertEqual(text.count(new),1);text=text.replace(new,old,1)
        self.assertEqual(hashlib.sha256(text.encode()).hexdigest(),value['base_script_sha256'])
    def test_selection_key_recheck_and_ready_order(self):
        text=legacy_script()
        select=text.index('$PlutoBundle = Pick-Folder'); key=text.index('$lines = @('); cache=text.index("if (-not (Test-Path -LiteralPath $ready))")
        self.assertLess(select,key);self.assertLess(key,cache)
        self.assertIn('$lines = @($resources, $mixinsRevision, $PlutoDigest,',text)
        self.assertEqual(text.count('Pluto-Input-Digest $PlutoBundle'),3)
        self.assertLess(text.index('Pluto inputs changed during the build'),text.index('Set-Content -LiteralPath $ready'))
        self.assertLess(text.index('Pluto inputs changed before launch'),text.index("try { Run $game @('--resources'"))
        self.assertEqual(text.count("$PlutoBundle = Pick-Folder"),1)
        self.assertIn("$digest.Count -ne 1",text)
        self.assertIn("$digest[0] -cnotmatch",text)

if __name__=='__main__':unittest.main()
