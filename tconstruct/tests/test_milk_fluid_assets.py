"""Validate original Forge milk pixels without rendering or network access."""
import hashlib
import json
from pathlib import Path
import struct
import unittest

ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / 'assets/forge/textures/block'


class MilkFluidAssets(unittest.TestCase):
    def test_pinned_forge_pixels_and_animation_are_unchanged(self):
        # MinecraftForge 243490d78001917c7e6ee6e94b45ef98776aa36f,
        # src/main/resources/assets/forge/textures/block/milk_still.*
        expected = {
            'milk_still.png': 'fc2fabbd47c86ac3a205a4b4925bde843e4ae2d3a40af75a75ed4dfd1d9db5cc',
            'milk_still.png.mcmeta': 'af915735740bd046c85e66fc03c47bb31517ef64f1721726151ea5de8cf121e3',
        }
        for name, digest in expected.items():
            self.assertEqual(hashlib.sha256((SOURCE / name).read_bytes()).hexdigest(), digest)
        png = (SOURCE / 'milk_still.png').read_bytes()
        self.assertEqual(png[:8], b'\x89PNG\r\n\x1a\n')
        self.assertEqual(struct.unpack('>II', png[16:24]), (16, 512))
        self.assertEqual(json.loads((SOURCE / 'milk_still.png.mcmeta').read_text()),
                         {'animation': {'frametime': 3}})


if __name__ == '__main__':
    unittest.main()
