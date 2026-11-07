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
        ids = {i['id'] for i in manifest['injection']}
        self.assertFalse(ids & {'source-host-mining-context', 'source-client-mining-context', 'tinkers-chest-pick-color'})
        self.assertGreater(len(ids), 250)
        # the launcher cannot work around Mixins' printing bug in these Horizon files, so nothing that edits them ships
        self.assertFalse([m for m in manifest['module'] if m['file'].endswith('worldgen/structure/start.rs')])


if __name__ == '__main__':
    unittest.main()
