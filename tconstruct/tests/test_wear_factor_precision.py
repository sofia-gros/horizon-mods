"""Discover the cached-library wear-factor harness without building dependencies."""
import os
import pathlib
import shutil
import subprocess
import tempfile
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[1]

class WearFactorPrecision(unittest.TestCase):
    def test_active_melee_paths_use_helper(self):
        for filename in ('material-melee-bonus.rs', 'source-melee-damage-return.rs'):
            self.assertIn('::wear_damage_bonus(', (ROOT / 'mixins/tconstruct' / filename).read_text())

    def test_cached_library_harness(self):
        directory = os.environ.get('TC_WEAR_HARNESS_DEPS')
        if not directory:
            self.skipTest('set TC_WEAR_HARNESS_DEPS to existing compatible Horizon rlibs')
        compiler = os.environ.get('RUSTC') or shutil.which('rustc')
        if not compiler:
            self.skipTest('rustc is required for the cached-library harness')
        deps = pathlib.Path(directory)
        command = [compiler, '--edition=2021', '--test', str(ROOT / 'tests/rust/wear_factor_precision.rs'), '-L', f'dependency={deps}']
        for crate in ('horizon_items', 'horizon_world', 'serde_json'):
            files = list(deps.glob(f'lib{crate}-*.rlib'))
            self.assertEqual(len(files), 1, f'provide an unambiguous cached {crate} library')
            command.extend(['--extern', f'{crate}={files[0]}'])
        with tempfile.TemporaryDirectory() as tmp:
            binary = pathlib.Path(tmp) / 'wear-factor-precision'
            subprocess.run(command + ['-o', str(binary)], check=True)
            subprocess.run([str(binary)], check=True)

if __name__ == '__main__':
    unittest.main()
