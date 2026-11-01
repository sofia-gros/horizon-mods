import json
import unittest
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]

class WorktableRecipe(unittest.TestCase):
    def test_stone_worktable_uses_original_pattern_and_inputs(self):
        with zipfile.ZipFile(ROOT / 'content/reference/tconstruct-recipes.zip') as archive:
            source = json.loads(archive.read('data/tconstruct/recipes/tables/modifier_worktable.json'))
            extract = json.loads(archive.read('data/tconstruct/recipes/tools/modifiers/worktable/extract/upgrade.json'))
        local = json.loads((ROOT / 'content/data/tconstruct/recipe/modifier_worktable_stone.json').read_text())
        self.assertEqual(local['pattern'], source['pattern'])
        self.assertEqual(local['key'], {'s': 'tconstruct:seared_bricks', 'r': 'minecraft:stone'})
        self.assertEqual(local['result']['id'], source['result']['item'])
        self.assertEqual([entry['item'] for entry in extract['inputs']], ['tconstruct:sky_slime_crystal', 'minecraft:wet_sponge'])
        self.assertEqual(extract['leftovers'], ['minecraft:sponge'])
