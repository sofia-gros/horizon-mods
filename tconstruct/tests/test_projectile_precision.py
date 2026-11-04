"""Discover the cached-library projectile harness without building dependencies."""
import os
import pathlib
import shutil
import subprocess
import tempfile
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[1]

class ProjectilePrecision(unittest.TestCase):
    def test_firing_paths_share_conditional_helper(self):
        for filename in ('material-bow-release.rs', 'material-crossbow-use.rs'):
            self.assertIn('projectile_bonus_at(stack, health, maximum)', (ROOT / 'mixins/tconstruct' / filename).read_text())
        self.assertIn('*base_damage += damage', (ROOT / 'mixins/tconstruct/source-projectile-add.rs').read_text())

    def test_cached_library_harness(self):
        directory = os.environ.get('TC_PROJECTILE_HARNESS_DEPS')
        if not directory:
            self.skipTest('set TC_PROJECTILE_HARNESS_DEPS to existing compatible Horizon rlibs')
        compiler = os.environ.get('RUSTC') or shutil.which('rustc')
        if not compiler:
            self.skipTest('rustc is required for the cached-library harness')
        deps = pathlib.Path(directory)
        command = [compiler, '--edition=2021', '--test', str(ROOT / 'tests/rust/projectile_precision.rs'), '-L', f'dependency={deps}']
        for crate in ('horizon_items', 'horizon_world'):
            files = list(deps.glob(f'lib{crate}-*.rlib'))
            self.assertEqual(len(files), 1, f'provide an unambiguous cached {crate} library')
            command.extend(['--extern', f'{crate}={files[0]}'])
        with tempfile.TemporaryDirectory() as tmp:
            binary = pathlib.Path(tmp) / 'projectile-precision'
            subprocess.run(command + ['-o', str(binary)], check=True)
            subprocess.run([str(binary)], check=True)

if __name__ == '__main__':
    unittest.main()
