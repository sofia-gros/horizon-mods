import hashlib
import json
from pathlib import Path
import unittest
import sys
import zipfile

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / 'tools'))
from piggy_labels import WARNINGS, mark_unavailable

class PiggyBackpackPartial(unittest.TestCase):
    def test_localized_warning_replaces_unsupported_instruction(self):
        self.assertEqual(len(WARNINGS), 20)
        for code in WARNINGS:
            text = mark_unavailable({'item.tconstruct.piggy_backpack': 'Name',
                                     'item.tconstruct.piggy_backpack.tooltip': 'Use on a mob'}, code)
            self.assertIn(WARNINGS[code], text['item.tconstruct.piggy_backpack'])
            self.assertEqual(text['item.tconstruct.piggy_backpack.tooltip'], WARNINGS[code])
        self.assertEqual(WARNINGS['ja_jp'], 'Mobの運搬は未対応')

    def test_pinned_casting_and_native_declaration(self):
        with zipfile.ZipFile(ROOT / 'content/reference/tconstruct-recipes.zip') as archive:
            recipe = json.loads(archive.read('data/tconstruct/recipes/gadgets/piggy_backpack.json'))
        self.assertEqual(recipe['type'], 'tconstruct:casting_table')
        self.assertEqual(recipe['cast'], {'item': 'minecraft:saddle'})
        self.assertTrue(recipe['cast_consumed'])
        self.assertEqual(recipe['fluid'], {'amount': 1000, 'tag': 'tconstruct:sky_slime'})
        self.assertEqual(recipe['result'], 'tconstruct:piggy_backpack')
        groups = json.loads((ROOT / 'content/creative-tabs.json').read_text())
        tools = next(group for group in groups if group['id'] == 'tconstruct:tools')
        self.assertEqual(tools['items'].count('{count:1,id:"tconstruct:piggy_backpack"}'), 1)
        declaration = json.loads((ROOT / 'content/data/tconstruct/horizon/item/piggy_backpack.json').read_text())
        self.assertEqual(declaration['max_stack_size'], 16)
        self.assertEqual(declaration['components']['minecraft:equippable']['slot'], 'chest')
        self.assertNotIn('minecraft:attribute_modifiers', declaration['components'])
        icon = (ROOT / 'content/assets/tconstruct/textures/item/gadgets/piggy_backpack.png').read_bytes()
        self.assertEqual(hashlib.sha256(icon).hexdigest(), 'c5efa0a714e507dcad12834ec483b0fbc628cd1cba2ec88ce366151bd97e2bff')

if __name__ == '__main__': unittest.main()
