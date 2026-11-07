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


if __name__ == '__main__':
    unittest.main()
