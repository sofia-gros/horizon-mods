"""Check the registered meat soup item and pinned fluid tag."""
import hashlib
import json
from pathlib import Path
import unittest
from zipfile import ZipFile

ROOT = Path(__file__).resolve().parents[1]

class MeatSoupContent(unittest.TestCase):
    def test_item_and_pinned_fluid_tag(self):
        item = json.loads((ROOT / 'content/data/tconstruct/horizon/item/meat_soup.json').read_text())
        self.assertEqual(item['template'], 'minecraft:mushroom_stew')
        self.assertEqual(item['max_stack_size'], 1)
        components = item['components']
        self.assertEqual(components['minecraft:food'], {'nutrition': 8, 'saturation': 9.6})
        self.assertEqual((components['minecraft:consumable']['animation'], components['minecraft:consumable']['consume_seconds']), ('drink', 1.6))
        self.assertEqual(components['minecraft:use_remainder'], {'id': 'minecraft:bowl'})
        self.assertEqual(item['crafting_remainder'], 'minecraft:bowl')
        self.assertNotIn('minecraft:crafting_remainder', components)
        model = json.loads((ROOT / 'content/assets/tconstruct/models/item/meat_soup.json').read_text())
        self.assertEqual(model['textures']['layer0'], 'tconstruct:item/food/meat_soup')
        self.assertTrue((ROOT / 'content/assets/tconstruct/items/meat_soup.json').is_file())
        sprite = ROOT / 'content/assets/tconstruct/textures/item/food/meat_soup.png'
        self.assertEqual(hashlib.sha256(sprite.read_bytes()).hexdigest(), 'b808685f7ad6e7e0081a5d1ff956a21a81499e1863488731061f542c71b8bf8f')
        manual = (ROOT / 'tools/resources.py').read_text()
        runtime = (ROOT / 'mixins/tconstruct/resource-tables.rs').read_text()
        self.assertIn("('tconstruct:meat_soup', 'minecraft:mushroom_stew', 'Meat Soup')", manual)
        for source in (manual, runtime):
            self.assertIn('minecraft:use_remainder', source)
            self.assertIn('minecraft:crafting_remainder', source)
            self.assertIn('9.6', source)
        with ZipFile(ROOT / 'content/reference/tconstruct-fluid-tags.zip') as archive:
            tag = json.loads(archive.read('data/tconstruct/tags/fluids/meat_soup.json'))
        self.assertEqual(tag['values'], ['tconstruct:meat_soup', 'tconstruct:flowing_meat_soup'])

if __name__ == '__main__':
    unittest.main()
