"""Exercise the actual tracked native bundle against the official loader budget."""
import importlib.util
import json
from pathlib import Path
import tempfile
import tomllib
import unittest
import zipfile

MOD = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location('tconstruct_package_hnmod', MOD / 'tools/package_hnmod.py')
PACKAGER = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(PACKAGER)


class NativePackageBudgetTest(unittest.TestCase):
    def test_actual_bundle_fits_official_limits_and_preserves_sources(self):
        # Do not substitute a source-file count: the manifest and support archive
        # also consume entries, and the packaged native ID rewrites matter.
        self.assertEqual(PACKAGER.MAX_ENTRIES, 512)
        self.assertEqual(PACKAGER.MAX_UNPACKED, 64 * 1024 * 1024)
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / 'tconstruct.hnmod'
            PACKAGER.package(output)
            with zipfile.ZipFile(output) as archive:
                entries = archive.infolist()
                self.assertLessEqual(len(entries), 512)
                self.assertLessEqual(sum(entry.file_size for entry in entries), 64 * 1024 * 1024)
                self.assertEqual(len(entries), len(set(archive.namelist())))
                self.assertEqual(json.loads(archive.read('horizon-native.json'))['id'], 'tconstruct_native')
                prefix = 'mixins/tconstruct_native/'
                config = tomllib.loads(archive.read(prefix + 'horizon-mixin.toml').decode())
                original = tomllib.loads((MOD / 'mixins/tconstruct/horizon-mixin.toml').read_text())
                self.assertEqual(config['mod']['id'], 'tconstruct_native')
                for group in ('module', 'injection', 'bridge', 'state', 'crate'):
                    self.assertEqual(config.get(group), original.get(group))
                for group, key in (('module', 'source'), ('injection', 'helper')):
                    for registration in config[group]:
                        path = registration[key]
                        expected = (MOD / 'mixins/tconstruct' / path).read_bytes().replace(
                            b'__horizon_mixins_tconstruct_', b'__horizon_mixins_tconstruct_native_')
                        self.assertEqual(archive.read(prefix + path), expected, registration['id'])
                self.assertIn(prefix + 'data/tconstruct-support.zip', archive.namelist())


if __name__ == '__main__':
    unittest.main()
