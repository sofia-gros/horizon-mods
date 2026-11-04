"""Bounded seared reinforcement asset and pinned recipe checks."""
import hashlib
import json
from pathlib import Path
import unittest
from zipfile import ZipFile

ROOT = Path(__file__).resolve().parents[1]

class SearedReinforcement(unittest.TestCase):
    def test_pinned_recipe_and_resources(self):
        fixture = json.loads((ROOT / 'tests/fixtures/seared_reinforcement.json').read_text())
        with ZipFile(ROOT / 'content/reference/tconstruct-recipes.zip') as archive:
            source = json.loads(archive.read('data/tconstruct/recipes/tools/modifiers/seared_reinforcement.json'))
        self.assertEqual(fixture, source)
        self.assertEqual(fixture['cast'], {'item': 'tconstruct:pattern'})
        self.assertTrue(fixture['cast_consumed'])
        self.assertEqual(fixture['cooling_time'], 89)
        self.assertEqual(fixture['fluid'], [{'amount': 250, 'tag': 'tconstruct:seared_stone'}, {'amount': 250, 'tag': 'tconstruct:scorched_stone'}])
        self.assertEqual(fixture['result'], 'tconstruct:seared_reinforcement')
        item = json.loads((ROOT / 'content/data/tconstruct/horizon/item/seared_reinforcement.json').read_text())
        self.assertEqual(item['template'], 'minecraft:brick')
        item_def = json.loads((ROOT / 'content/assets/tconstruct/items/seared_reinforcement.json').read_text())
        self.assertEqual(item_def['model']['model'], 'tconstruct:item/seared_reinforcement')
        model = json.loads((ROOT / 'content/assets/tconstruct/models/item/seared_reinforcement.json').read_text())
        self.assertEqual(model['textures']['layer0'], 'tconstruct:item/materials/reinforcement/seared')
        texture = ROOT / 'content/assets/tconstruct/textures/item/materials/reinforcement/seared.png'
        self.assertEqual(hashlib.sha256(texture.read_bytes()).hexdigest(), 'cdd283ea1caa70d449569beb59cabd4f2ca32c5b895148d7a18168625e3bbba8')
        self.assertTrue((ROOT / 'third_party/tconstruct/LICENSE').is_file())

if __name__ == '__main__':
    unittest.main()
