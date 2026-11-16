"""Run the production contact adapter against a trace-only host double.

The double makes native hurt teleport immediately; it does not emulate damage
or certify Horizon gameplay. The regression checks adapter call order/path.
"""
from pathlib import Path
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]


class MobInsideTests(unittest.TestCase):
    def test_native_hurt_and_pre_contact_sweep_survive_teleport(self):
        fixture = (ROOT / 'pluto-lib/tests/fixtures/mob_inside.rs').read_text()
        helper = (ROOT / 'pluto-lib/mod/pluto-mob-inside.rs').as_posix()
        with tempfile.TemporaryDirectory(prefix='mob-inside-') as temp:
            source = Path(temp) / 'contact.rs'
            binary = Path(temp) / 'contact.exe'
            source.write_text(newline='\n', data=fixture.replace('PRODUCTION_HELPER', '"' + helper + '"'), encoding='utf-8')
            subprocess.run(['rustc', '+1.95.0', '--edition=2021', '--test', str(source), '-o', str(binary)], check=True)
            subprocess.run([str(binary)], check=True)


if __name__ == '__main__':
    unittest.main()
