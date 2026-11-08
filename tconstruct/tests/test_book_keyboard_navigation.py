"""Narrow guide-key adapter, exercised without a window or real LAN session."""
from pathlib import Path
import shutil
import subprocess
import tempfile
import tomllib
import unittest

ROOT = Path(__file__).resolve().parents[1]


class GuideKeyboardNavigation(unittest.TestCase):

    @unittest.skipUnless(shutil.which('rustc'), 'rustc unavailable for bounded adapter fixture')
    def test_production_keyboard_and_turn_adapters(self):
        with tempfile.TemporaryDirectory(prefix='book-keyboard-') as tmp:
            binary = Path(tmp) / 'book-keyboard-tests'
            subprocess.run(['rustc', '--edition=2021', '--test', str(ROOT / 'tests/rust/book_keyboard_navigation.rs'), '-o', str(binary)], check=True)
            subprocess.run([str(binary)], check=True)


if __name__ == '__main__':
    unittest.main()
