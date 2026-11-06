"""Source guards only. Does not compile or execute the prepared Rust module."""
import hashlib,json,re,unittest
from pathlib import Path
HERE=Path(__file__).resolve().parent
class SourceChecks(unittest.TestCase):
    def test_frozen_group_bytes(self):
        meta=json.loads((HERE/'inputs.json').read_text())
        for name,digest in meta['source_sha256'].items():
            self.assertEqual(hashlib.sha256((HERE/name).read_bytes()).hexdigest(),digest)
        self.assertEqual(meta['official'],'624c4f6265c02296fbe8a80f029dd0d987fb94c0')
        self.assertEqual(meta['status'],'source_prepared_rust_unrun')
    def test_actual_imports_and_exact_four_names(self):
        meta=json.loads((HERE/'inputs.json').read_text());source=(HERE/'catalog_tests.rs').read_text()
        self.assertEqual(re.findall(r'#\[test\]\s*fn ([a-z0-9_]+)',source),meta['tests'])
        self.assertEqual(len(meta['tests']),4)
        for required in ['use horizon_items::','use horizon_world::','Items::catalog','item_defaults::shared','item_defaults::read','encode_components']:
            self.assertIn(required,source)
        for forbidden in ['native_load_hooks::install','thread_local!','horizon-mixin','impl Items']:
            self.assertNotIn(forbidden,source)
    def test_no_manifest_or_new_ci_and_no_arc_lifetime_claim(self):
        self.assertEqual({p.name for p in HERE.iterdir() if p.is_file()},{'catalog_tests.rs','README.md','inputs.json','test_source.py'})
        readme=(HERE/'README.md').read_text()
        for required in ['No global map','release the native Arc','no constructor-local capture','not wired into codec32','not Rust execution']:
            self.assertIn(required,readme)
if __name__=='__main__':unittest.main()
