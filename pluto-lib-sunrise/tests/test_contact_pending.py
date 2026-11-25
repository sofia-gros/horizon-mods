"""Execute the production transient queue's clock, clone and capacity rules."""
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]


class ContactPendingTests(unittest.TestCase):
    @unittest.skipUnless(shutil.which('rustc'), 'Rust toolchain is required')
    def test_transient_effect_lifetime_and_capacity(self):
        with tempfile.TemporaryDirectory(prefix='pluto-contact-pending-') as folder:
            binary = Path(folder) / ('queue.exe' if os.name == 'nt' else 'queue')
            subprocess.run(['rustc', '--edition=2024', '--test',
                            str(ROOT / 'mod/pluto-contact-pending.rs'), '-o', str(binary)], check=True)
            subprocess.run([str(binary)], check=True)


if __name__ == '__main__':
    unittest.main()
