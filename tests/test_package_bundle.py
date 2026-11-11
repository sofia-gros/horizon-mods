import importlib.util
from pathlib import Path, PurePosixPath
import unittest

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location('package_bundle', ROOT / 'tools/package_bundle.py')
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


class BundleFileRules(unittest.TestCase):
    def test_only_what_horizon_accepts_in_a_bundle_is_packed(self):
        ok = ['horizon-mixin.toml', 'api/src/lib.rs', 'api/Cargo.toml', 'data/x/y.json', 'engine/data/z.json', 'README.md', 'LICENSE']
        bad = ['api/Cargo.lock', 'notes.md', 'api/target/debug/a.exe', 'image.png', 'src/data.json']
        for name in ok:
            self.assertTrue(module.allowed(PurePosixPath(name)), name)
        for name in bad:
            self.assertFalse(module.allowed(PurePosixPath(name)), name)


class ProductNotices(unittest.TestCase):
    def test_archive_keeps_the_product_notice_beside_its_bundle(self):
        import json, tempfile, zipfile
        from unittest.mock import patch
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            bundle = root / 'product/mod'
            bundle.mkdir(parents=True)
            (root / 'tconstruct').mkdir()
            (root / 'tconstruct/compatibility.json').write_text(json.dumps({'horizon_revision':'pin','mixins_revision':'mixins'}), encoding='utf-8', newline='\n')
            manifest = bundle / 'horizon-mixin.toml'
            manifest.write_text('[mod]\nid="example"\nversion="1"\n[compatibility]\nhorizon_revision="pin"\n', encoding='utf-8', newline='\n')
            notice = bundle.parent / 'LICENSE.horizon'
            notice.write_bytes(b'Fixture upstream permission notice\n')
            unrelated = bundle.parent / 'other/LICENSE'
            unrelated.parent.mkdir()
            unrelated.write_bytes(b'Other product notice\n')
            def tracked(path):
                return [manifest] if path == bundle else [manifest,notice,unrelated]
            out = root / 'example.hnmod'
            with patch.object(module,'tracked_files',side_effect=tracked):
                module.build(bundle,out,root)
            with zipfile.ZipFile(out) as archive:
                self.assertEqual(archive.read('LICENSE.horizon'),notice.read_bytes())
                self.assertNotIn('LICENSE',archive.namelist())
                self.assertIn('mixins/example/horizon-mixin.toml',archive.namelist())


class TinkersPackageSlice(unittest.TestCase):
    def test_packaged_manifest_leaves_out_the_hooks_mixins_refuses(self):
        import tempfile, tomllib, zipfile
        spec = importlib.util.spec_from_file_location('package_hnmod', ROOT / 'tconstruct/tools/package_hnmod.py')
        pkg = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(pkg)
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp) / 'tconstruct.hnmod'
            pkg.package(out)
            with zipfile.ZipFile(out) as z:
                manifest = tomllib.loads(z.read('mixins/tconstruct_native/horizon-mixin.toml').decode())
                names = set(z.namelist())
                self.assertLessEqual(len(names), pkg.MAX_ENTRIES)
                for kind, key in [('module', 'source'), ('injection', 'helper')]:
                    for entry in manifest.get(kind, []):
                        if key in entry:
                            self.assertIn('mixins/tconstruct_native/' + entry[key], names)
                self.assertIn('mixins/tconstruct_native/api/src/lib.rs', names)
        ids = {i['id'] for i in manifest['injection']}
        self.assertFalse(ids & {'source-host-mining-context', 'source-client-mining-context', 'tinkers-chest-pick-color'})
        self.assertGreater(len(ids), 100)  # most injections ship; the count falls as they move onto PlutoLib
        # the launcher cannot work around Mixins' printing bug in these Horizon files, so nothing that edits them ships
        self.assertFalse([m for m in manifest['module'] if m['file'].endswith('worldgen/structure/start.rs')])


if __name__ == '__main__':
    unittest.main()
