"""Execute real mining arithmetic without building host dependencies."""
import pathlib
import shutil
import subprocess
import tempfile
import unittest
ROOT = pathlib.Path(__file__).resolve().parents[1]

class MiningFactorPrecision(unittest.TestCase):
    def test_real_arithmetic(self):
        compiler = shutil.which('rustc')
        if not compiler:
            self.skipTest('rustc is required for the mining factor harness')
        with tempfile.TemporaryDirectory() as tmp:
            binary = pathlib.Path(tmp) / 'mining-factors'
            subprocess.run([compiler, '--edition=2021', '--test', str(ROOT / 'tests/rust/mining_factor_precision.rs'), '-o', str(binary)], check=True)
            subprocess.run([str(binary)], check=True)


if __name__ == '__main__':
    unittest.main()
