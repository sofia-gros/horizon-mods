"""Execute the actual release helper using cached Horizon dependencies only."""
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest
ROOT = Path(__file__).resolve().parents[1]
class LongbowReleaseThreshold(unittest.TestCase):
    def test_active_release_hook(self):
        manifest = (ROOT / 'mixins/tconstruct/horizon-mixin.toml').read_text()
        rule = manifest.split('id = "material-bow-release"', 1)[1].split('[[', 1)[0]
        self.assertIn('helper = "material-bow-release.rs"', rule)
        self.assertIn('::release_used_item"', rule)
        self.assertIn('kind = "HEAD"', rule)
        self.assertIn('cancellable = true', rule)
    def test_active_crossbow_charge_hook(self):
        manifest = (ROOT / 'mixins/tconstruct/horizon-mixin.toml').read_text()
        rule = manifest.split('id = "material-crossbow-charge"', 1)[1].split('[[', 1)[0]
        self.assertIn('helper = "material-crossbow-charge.rs"', rule)
        self.assertIn('kind = "HEAD"', rule)
        self.assertIn('cancellable = true', rule)
    def test_cached_release_harness(self):
        directory = os.environ.get('TC_LONGBOW_HARNESS_DEPS')
        if not directory:
            self.skipTest('set TC_LONGBOW_HARNESS_DEPS to compatible cached Horizon rlibs')
        compiler = os.environ.get('RUSTC') or shutil.which('rustc')
        if not compiler:
            self.skipTest('rustc is required for the release harness')
        deps = Path(directory)
        command = [compiler, '--edition=2021', '--test', str(ROOT / 'tests/rust/longbow_release_threshold.rs'), '-L', f'dependency={deps}', '-C', 'debuginfo=0', '-C', 'strip=debuginfo']
        for crate in ('horizon_items', 'horizon_world'):
            files = list(deps.glob(f'lib{crate}-*.rlib'))
            self.assertEqual(len(files), 1, f'provide an unambiguous cached {crate} library')
            command.extend(['--extern', f'{crate}={files[0]}'])
        with tempfile.TemporaryDirectory() as tmp:
            binary = Path(tmp) / 'longbow-release'
            subprocess.run(command + ['-o', str(binary)], check=True)
            subprocess.run([str(binary)], check=True)

if __name__ == '__main__':
    unittest.main()
