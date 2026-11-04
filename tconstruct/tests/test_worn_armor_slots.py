"""Execute the actual equipment helper with small host boundary doubles."""
import pathlib
import shutil
import subprocess
import tempfile
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[1]

class WornArmorSlots(unittest.TestCase):
    def test_equipment_helper(self):
        if not shutil.which('rustc'):
            self.skipTest('rustc is required for the equipment boundary harness')
        with tempfile.TemporaryDirectory() as tmp:
            binary = pathlib.Path(tmp) / 'worn-armor-slots'
            subprocess.run(['rustc', '--edition=2021', '--test', str(ROOT / 'tests/rust/worn_armor_slots.rs'), '-o', str(binary)], check=True)
            subprocess.run([str(binary)], check=True)

    def test_runtime_hook_is_enabled(self):
        manifest = (ROOT / 'mixins/tconstruct/horizon-mixin.toml').read_text()
        injection = manifest.split('id = "material-armor-attributes"', 1)[1].split('[[', 1)[0]
        self.assertIn('apply_enchantment_attributes', injection)
        self.assertIn('kind = "HEAD"', injection)
        self.assertIn('helper = "material-armor-attributes.rs"', injection)

if __name__ == '__main__':
    unittest.main()
