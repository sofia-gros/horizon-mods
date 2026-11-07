import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
import zipfile

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location('validate_release', ROOT / 'tools/validate_release.py')
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


class ReleaseValidationTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.path = Path(self.tmp.name)
        self.compat = json.loads((ROOT / 'tconstruct/compatibility.json').read_text())
        self.pluto = json.loads((ROOT / 'pluto-lib/horizon-native.json').read_text())
        self.native = dict(self.pluto, id='tconstruct_native', version='0.5.0')
        self.write_packages()

    def archive(self, name, entries):
        with zipfile.ZipFile(self.path / name, 'w') as z:
            for key, value in entries.items():
                z.writestr(key, value)

    def mixin(self, manifest):
        return (f'[mod]\nid = "{manifest["id"]}"\nversion = "{manifest["version"]}"\n'
                f'[compatibility]\nhorizon_revision = "{manifest["horizon"]}"\n')

    def write_packages(self):
        self.archive('tconstruct.hmod', {'horizon-mod.toml': 'id = "tconstruct"\nversion = "0.5.0"\n',
                    'assets/tconstruct/a': 'a', 'data/tconstruct/a': 'a'})
        for filename, manifest in [('tconstruct.hnmod', self.native), ('plutolib.hnmod', self.pluto)]:
            self.archive(filename, {'horizon-native.json': json.dumps(manifest),
                         f'mixins/{manifest["id"]}/horizon-mixin.toml': self.mixin(manifest)})

    def test_correct_packages(self):
        module.validate('tconstruct-v0.5.0', self.path)

    def test_wrong_tag_fails(self):
        with self.assertRaises(ValueError):
            module.validate('tconstruct-v0.5.1', self.path)

    def test_mismatched_pins_fail(self):
        self.native['mixins'] = 'bad'
        self.write_packages()
        with self.assertRaises(ValueError):
            module.validate('tconstruct-v0.5.0', self.path)

    def test_wrong_pluto_metadata_fails(self):
        self.pluto['version'] = '9.0.0'
        self.write_packages()
        with self.assertRaises(ValueError):
            module.validate('tconstruct-v0.5.0', self.path)

    def test_pluto_lib_release(self):
        source = module.tomllib.loads((ROOT / 'pluto-lib/mod/horizon-mixin.toml').read_text())
        manifest = {'schema': 1, 'id': 'pluto_lib', 'version': source['mod']['version'], 'game': '26.3',
                    'horizon': self.compat['horizon_revision'], 'mixins': self.compat['mixins_revision']}
        self.archive('pluto-lib.hnmod', {'horizon-native.json': json.dumps(manifest),
                     'mixins/pluto_lib/horizon-mixin.toml': (ROOT / 'pluto-lib/mod/horizon-mixin.toml').read_text()})
        self.assertEqual(module.validate(f'pluto-lib-v{manifest["version"]}', self.path), 'pluto-lib')
        with self.assertRaises(ValueError):
            module.validate('pluto-lib-v9.9.9', self.path)

    def test_untagged_product_is_refused(self):
        with self.assertRaises(ValueError):
            module.validate('v0.5.0', self.path)


if __name__ == '__main__':
    unittest.main()
