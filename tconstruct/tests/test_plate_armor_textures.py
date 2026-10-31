"""The retained source snapshot supplies paired steel and cobalt helmet layers."""
import hashlib
import unittest
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]

class PlateArmorTextures(unittest.TestCase):
    def test_representative_source_layers(self):
        archive = ROOT / 'content/reference/tconstruct-plate-armor-textures.zip'
        originals = ROOT / 'content/assets/tconstruct/textures/tinker_armor/plate'
        with zipfile.ZipFile(archive) as source:
            for material in ('steel', 'cobalt'):
                for kind in ('plating', 'maille'):
                    name = f'{kind}_armor_tconstruct_{material}.png'
                    data = source.read(name)
                    self.assertEqual(data[:8], b'\x89PNG\r\n\x1a\n')
                    if material == 'steel':
                        self.assertEqual(hashlib.sha256(data).digest(), hashlib.sha256((originals / name).read_bytes()).digest())

if __name__ == '__main__':
    unittest.main()
