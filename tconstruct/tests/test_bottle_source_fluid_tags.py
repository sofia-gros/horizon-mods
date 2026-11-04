"""Check bundled fluid IDs for the bounded source bottle fills."""
import json
from pathlib import Path
import unittest
from zipfile import ZipFile

ROOT = Path(__file__).resolve().parents[1]

class BottleSourceFluidTags(unittest.TestCase):
    def test_bundled_source_tags(self):
        with ZipFile(ROOT / 'content/reference/tconstruct-fluid-tags.zip') as archive:
            for fluid in ('magma', 'earth_slime', 'sky_slime', 'ichor', 'ender_slime'):
                with self.subTest(fluid=fluid):
                    tag = json.loads(archive.read(f'data/tconstruct/tags/fluids/{fluid}.json'))
                    self.assertEqual(tag['values'], [f'tconstruct:{fluid}', f'tconstruct:flowing_{fluid}'])

if __name__ == '__main__':
    unittest.main()
