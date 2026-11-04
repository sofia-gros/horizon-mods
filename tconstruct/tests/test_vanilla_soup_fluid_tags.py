"""Check the bundled fluid IDs used by the bounded Mantle bowl fills."""
import json
from pathlib import Path
import unittest
from zipfile import ZipFile

ROOT = Path(__file__).resolve().parents[1]

class VanillaSoupFluidTags(unittest.TestCase):
    def test_bundled_source_tags(self):
        with ZipFile(ROOT / 'content/reference/tconstruct-fluid-tags.zip') as archive:
            for food in ('beetroot_soup', 'mushroom_stew', 'rabbit_stew'):
                with self.subTest(food=food):
                    forge = json.loads(archive.read(f'data/forge/tags/fluids/{food}.json'))
                    source = json.loads(archive.read(f'data/tconstruct/tags/fluids/{food}.json'))
                    self.assertEqual(forge['values'], [f'#tconstruct:{food}'])
                    self.assertEqual(source['values'], [f'tconstruct:{food}', f'tconstruct:flowing_{food}'])

if __name__ == '__main__':
    unittest.main()
