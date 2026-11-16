"""Production spawn helper trace: yaw policy, own RNG and UUID before join."""
from pathlib import Path
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]


class RuntimeSpawnTests(unittest.TestCase):
    def test_native_identity_and_yaw(self):
        fixture = (ROOT / 'pluto-lib/tests/fixtures/runtime_spawn.rs').read_text()
        helper = (ROOT / 'pluto-lib/mod/pluto-entity-spawn-runtime-head.rs').as_posix()
        with tempfile.TemporaryDirectory(prefix='runtime-spawn-') as temp:
            source = Path(temp) / 'spawn.rs'
            binary = Path(temp) / 'spawn.exe'
            source.write_text(newline='\n', data=fixture.replace('PRODUCTION_HELPER', '"' + helper + '"'), encoding='utf-8')
            subprocess.run(['rustc', '+1.95.0', '--edition=2021', '--test', str(source), '-o', str(binary)], check=True)
            subprocess.run([str(binary)], check=True)


if __name__ == '__main__':
    unittest.main()
