"""Execute the production restitution adapter with host-supplied contacts.

This checks callback routing and native fallback, not collision or game parity.
"""
from pathlib import Path
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]


class ItemAfterFallTests(unittest.TestCase):
    def test_native_contact_routing(self):
        fixture = (ROOT / 'pluto-lib/tests/fixtures/item_after_fall.rs').read_text()
        helper = (ROOT / 'pluto-lib/mod/pluto-item-after-fall.rs').as_posix()
        with tempfile.TemporaryDirectory(prefix='item-after-fall-') as temp:
            source = Path(temp) / 'contact.rs'
            binary = Path(temp) / 'contact.exe'
            source.write_text(newline='\n', data=fixture.replace('PRODUCTION_HELPER', '"' + helper + '"'), encoding='utf-8')
            subprocess.run(['rustc', '+1.95.0', '--edition=2021', '--test', str(source), '-o', str(binary)], check=True)
            subprocess.run([str(binary)], check=True)


if __name__ == '__main__':
    unittest.main()
