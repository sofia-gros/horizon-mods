"""Verify source output/count and the chosen native 26.3 sand inputs."""
import json
from pathlib import Path
import unittest
import zipfile

ROOT = Path(__file__).resolve().parents[1]

class BlankCastRecipes(unittest.TestCase):
    def test_native_inputs_keep_source_output_and_count(self):
        with zipfile.ZipFile(ROOT / 'content/reference/tconstruct-recipes.zip') as archive:
            for kind in ('sand', 'red_sand'):
                original = json.loads(archive.read(f'data/tconstruct/recipes/smeltery/{kind}_cast.json'))
                native = json.loads((ROOT / f'content/data/tconstruct/recipe/blank_{kind}_cast.json').read_text())
                self.assertEqual(native['type'], original['type'])
                self.assertEqual(native['ingredients'], [f'minecraft:{kind}'])
                self.assertEqual(native['result'], {'id': original['result']['item'], 'count': original['result']['count']})
