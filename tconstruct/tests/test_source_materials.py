"""Pinned material definitions, stats, traits and tags stay byte-identical."""
import hashlib
from pathlib import Path
import unittest
import zipfile

ROOT = Path(__file__).resolve().parents[1]

class SourceMaterials(unittest.TestCase):
    def test_material_pack(self):
        pack = ROOT / 'content/reference/tconstruct-materials.zip'
        self.assertEqual(hashlib.sha256(pack.read_bytes()).hexdigest(), '5076dc235c89022d26f71e8c125ba073977e9c540adedb628c897fd7c657e6c7')
        with zipfile.ZipFile(pack) as archive:
            names = archive.namelist()
            self.assertEqual(len(names), 320)
            self.assertEqual(sum('/materials/definition/' in n for n in names), 104)
            self.assertEqual(sum('/materials/stats/' in n for n in names), 99)
            self.assertEqual(sum('/materials/traits/' in n for n in names), 99)
            self.assertEqual(sum('/tags/materials/' in n for n in names), 18)
