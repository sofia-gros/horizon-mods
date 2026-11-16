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
            (bundle / 'new_helper.rs').write_text('pub fn helper() {}', encoding='utf-8', newline='\n')
            rejected = root / 'incomplete.hnmod'
            with patch.object(module, 'tracked_files', side_effect=tracked):
                with self.assertRaisesRegex(ValueError, 'untracked source inputs'):
                    module.build(bundle, rejected, root)
            self.assertFalse(rejected.exists())
            manifest.write_text(manifest.read_text(encoding='utf-8') + '\n[[module]]\nid="helper"\nsource="new_helper.rs"\n', encoding='utf-8', newline='\n')
            (bundle / 'unrelated.rs').write_text('must not ship', encoding='utf-8', newline='\n')
            with patch.object(module, 'tracked_files', side_effect=tracked):
                module.build(bundle, rejected, root, manifest_inputs=True)
            with zipfile.ZipFile(rejected) as archive:
                self.assertIn('mixins/example/new_helper.rs', archive.namelist())
                self.assertNotIn('mixins/example/unrelated.rs', archive.namelist())
            manifest.write_text(manifest.read_text(encoding='utf-8').replace('new_helper.rs', '../escape.rs'), encoding='utf-8', newline='\n')
            with patch.object(module, 'tracked_files', side_effect=tracked):
                with self.assertRaisesRegex(ValueError, 'unsafe declared source'):
                    module.build(bundle, root / 'escape.hnmod', root, manifest_inputs=True)


class TinkersPackageBoundary(unittest.TestCase):
    def test_package_contains_the_exact_checked_inputs_without_host_hooks(self):
        import tempfile, tomllib, zipfile
        spec = importlib.util.spec_from_file_location('clean_package', ROOT / 'tools/port.py')
        pkg = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(pkg)
        expected, _, _, _ = pkg.checked_files(ROOT)
        (ROOT / '.local').mkdir(exist_ok=True)
        with tempfile.TemporaryDirectory(dir=ROOT / '.local') as tmp:
            out = Path(tmp) / 'tconstruct.hnmod'
            pkg.package(ROOT, out)
            with zipfile.ZipFile(out) as archive:
                prefix = 'mixins/tconstruct_native/'
                self.assertEqual(set(archive.namelist()), {'horizon-native.json'} | {prefix + name for name in expected})
                manifest = tomllib.loads(archive.read(prefix + 'horizon-mixin.toml').decode())
                self.assertFalse(set(manifest) & {'injection', 'module', 'state', 'bridge', 'crate'})
                self.assertEqual(manifest['dependencies'], {'pluto_lib': '^0.1'})
                for name, content in expected.items():
                    if name != 'horizon-mixin.toml':  # package identity is rewritten by the packager
                        self.assertEqual(archive.read(prefix + name), content)


if __name__ == '__main__':
    unittest.main()
