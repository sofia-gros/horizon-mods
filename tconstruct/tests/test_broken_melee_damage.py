"""Execute the real melee projection helper and retained broken predicate."""
import os
import pathlib
import shutil
import subprocess
import tempfile
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[1]

class BrokenMeleeDamage(unittest.TestCase):
    def test_projection_helper(self):
        if not shutil.which('rustc'):
            self.skipTest('rustc is required for the melee projection boundary harness')
        directory = os.environ.get('TC_MELEE_HARNESS_DEPS')
        if not directory:
            self.skipTest('set TC_MELEE_HARNESS_DEPS to compatible cached serde_json rlibs')
        deps = pathlib.Path(directory)
        libraries = list(deps.glob('libserde_json-*.rlib'))
        self.assertEqual(len(libraries), 1)
        with tempfile.TemporaryDirectory() as tmp:
            binary = pathlib.Path(tmp) / 'melee-projection'
            subprocess.run(['rustc', '--edition=2021', '--test', str(ROOT / 'tests/rust/broken_melee_damage.rs'), '-L', f'dependency={deps}', '--extern', f'serde_json={libraries[0]}', '-C', 'debuginfo=0', '-C', 'strip=debuginfo', '-o', str(binary)], check=True)
            subprocess.run([str(binary)], check=True)

    def test_active_hook(self):
        manifest = (ROOT / 'mixins/tconstruct/horizon-mixin.toml').read_text()
        injection = manifest.split('id = "material-melee-bonus"', 1)[1].split('[[', 1)[0]
        self.assertIn('::attack_damage', injection)
        self.assertIn('helper = "material-melee-bonus.rs"', injection)
        self.assertIn('kind = "HEAD"', injection)

if __name__ == '__main__':
    unittest.main()
