"""Exercise the injected material attack-speed helper, not a copied formula."""
import pathlib
import shutil
import subprocess
import tempfile
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[1]

class MaterialAttackSpeed(unittest.TestCase):
    def test_equipment_helper(self):
        if not shutil.which('rustc'):
            self.skipTest('rustc is required for the attack-speed boundary harness')
        with tempfile.TemporaryDirectory() as tmp:
            binary = pathlib.Path(tmp) / 'attack-speed'
            subprocess.run(['rustc', '--edition=2021', '--test', str(ROOT / 'tests/rust/material_attack_speed.rs'), '-o', str(binary)], check=True)
            subprocess.run([str(binary)], check=True)

    def test_runtime_hook_is_enabled(self):
        manifest = (ROOT / 'mixins/tconstruct/horizon-mixin.toml').read_text()
        injection = manifest.split('id = "material-attack-speed"', 1)[1].split('[[', 1)[0]
        self.assertIn('main_hand_attack_speed', injection)
        self.assertIn('helper = "material-attack-speed.rs"', injection)
        self.assertIn('kind = "HEAD"', injection)

if __name__ == '__main__':
    unittest.main()
