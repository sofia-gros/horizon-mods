"""Bounded packaging checks; these do not execute Windows or a host build."""
import json
from pathlib import Path
import sys
import tempfile
import unittest
import zipfile

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / 'tools'))
from package_hmod import package

class FoundryDistribution(unittest.TestCase):
    def test_native_package_keeps_declared_item_recipe_and_reader_assets(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'tconstruct.hmod'
            package(path)
            with zipfile.ZipFile(path) as archive:
                for item in (
                    'data/tconstruct/horizon/item/fantastic_foundry.json',
                    'data/tconstruct/recipe/fantastic_foundry.json',
                    'assets/tconstruct/book/fantastic_foundry/index.json',
                    'assets/tconstruct/book/fantastic_foundry/en_us/foundry/gui1.json',
                    'assets/tconstruct/book/fantastic_foundry/en_us/foundry/byproducts.json',
                    'assets/tconstruct/book/images/covers/fantastic_foundry_cover.png',
                    'assets/tconstruct/items/fantastic_foundry.json',
                ):
                    self.assertEqual(archive.read(item), (ROOT / 'content' / item).read_bytes())
                declared = json.loads(archive.read('data/tconstruct/horizon/item/fantastic_foundry.json'))
                self.assertEqual(declared['template'], 'minecraft:book')

    def test_windows_pipeline_applies_content_after_manual_overlay(self):
        script = (ROOT / 'tools/play-windows.ps1').read_text()
        overlay = script.index("'tools/resources.py'")
        prepare = script.index("'--tconstruct-prepare'", overlay)
        self.assertLess(overlay, prepare)
        self.assertIn("(Join-Path $Repo 'content')", script[prepare:])
        prepare_source = (ROOT / 'mixins/tconstruct/prepare.rs').read_text()
        self.assertIn('Content::read(&content_path, &manifest)', prepare_source)
        self.assertIn('mod_content::build(&base, &[content_mod], &out)', prepare_source)
        self.assertTrue((ROOT / 'content/data/tconstruct/horizon/item/fantastic_foundry.json').is_file())

if __name__ == '__main__':
    unittest.main()
