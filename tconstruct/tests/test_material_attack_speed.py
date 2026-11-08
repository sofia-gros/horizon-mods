"""Exercise the injected material attack-speed helper, not a copied formula."""
import os
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
        directory = os.environ.get('TC_ATTACK_SPEED_HARNESS_DEPS')
        if not directory:
            self.skipTest('set TC_ATTACK_SPEED_HARNESS_DEPS to compatible cached serde_json rlibs')
        deps = pathlib.Path(directory)
        libraries = list(deps.glob('libserde_json-*.rlib'))
        self.assertEqual(len(libraries), 1)
        with tempfile.TemporaryDirectory() as tmp:
            binary = pathlib.Path(tmp) / 'attack-speed'
            subprocess.run(['rustc', '--edition=2021', '--test', str(ROOT / 'tests/rust/material_attack_speed.rs'), '-L', f'dependency={deps}', '--extern', f'serde_json={libraries[0]}', '-C', 'debuginfo=0', '-C', 'strip=debuginfo', '-o', str(binary)], check=True)
            subprocess.run([str(binary)], check=True)


if __name__ == '__main__':
    unittest.main()
