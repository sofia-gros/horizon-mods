"""Run the actual Raging damage helper with the actual broken predicate."""
import pathlib
import shutil
import subprocess
import tempfile
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[1]

class RagingBroken(unittest.TestCase):
    def test_actual_damage_helper(self):
        if not shutil.which('rustc'):
            self.skipTest('rustc is required for the Raging boundary harness')
        with tempfile.TemporaryDirectory() as tmp:
            binary = pathlib.Path(tmp) / 'raging'
            subprocess.run(['rustc', '--edition=2021', '--test', str(ROOT / 'tests/rust/raging_broken.rs'), '-o', str(binary)], check=True)
            subprocess.run([str(binary)], check=True)

    def test_return_hook_is_enabled(self):
        manifest = (ROOT / 'mixins/tconstruct/horizon-mixin.toml').read_text()
        injection = manifest.split('id = "source-melee-damage-return"', 1)[1].split('[[', 1)[0]
        self.assertIn('::melee_damage', injection)
        self.assertIn('kind = "RETURN"', injection)
        self.assertIn('source-melee-damage-return.rs', injection)

if __name__ == '__main__':
    unittest.main()
