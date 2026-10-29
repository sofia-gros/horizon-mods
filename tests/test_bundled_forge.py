"""Exercise the packaged original Forge data without a Forge installation."""
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]

class BundledForge(unittest.TestCase):
    def test_extract_verified_data_and_refuse_existing_output(self):
        manifest = json.loads((ROOT / 'content/reference/forge-47.4.0-data.sha256.json').read_text())
        self.assertEqual(len(manifest), 359)
        with tempfile.TemporaryDirectory() as temp:
            out = Path(temp) / 'forge-pack'
            cmd = [sys.executable, str(ROOT / 'tools/prepare_bundled_forge.py'), str(out)]
            subprocess.run(cmd, check=True, capture_output=True)
            for name, digest in manifest.items():
                self.assertEqual(hashlib.sha256((out / name).read_bytes()).hexdigest(), digest, name)
            self.assertNotEqual(subprocess.run(cmd, capture_output=True).returncode, 0)
