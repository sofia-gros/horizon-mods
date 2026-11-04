"""Keep the partial tinted-glass guide page in the distributed content mod."""
import json
from pathlib import Path
import sys
import tempfile
import unittest
from zipfile import ZipFile

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / 'tools'))
from package_hmod import package

class MightyTintedPackage(unittest.TestCase):
    def test_adapted_page_is_packaged(self):
        name = 'assets/tconstruct/book/mighty_smelting/en_us/smeltery/tinted_glass.json'
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp) / 'tconstruct.hmod'
            package(out)
            with ZipFile(out) as packed:
                self.assertEqual(json.loads(packed.read(name)), json.loads((ROOT / 'content' / name).read_text()))

if __name__ == '__main__':
    unittest.main()
