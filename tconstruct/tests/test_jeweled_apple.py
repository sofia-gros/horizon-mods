"""Pinned Jeweled Apple casting and item data."""
import hashlib
import json
from pathlib import Path
import unittest
import zipfile

ROOT = Path(__file__).resolve().parents[1]

class JeweledApple(unittest.TestCase):
    def test_cast_and_components(self):
        with zipfile.ZipFile(ROOT / 'content/reference/tconstruct-recipes.zip') as z:
            recipe = json.loads(z.read('data/tconstruct/recipes/tools/modifiers/jeweled_apple.json'))
            gem = json.loads(z.read('data/tconstruct/recipes/smeltery/melting/diamond/gem.json'))
        self.assertEqual(recipe, {'type':'tconstruct:casting_table', 'cast':{'item':'minecraft:apple'},
            'cast_consumed':True, 'cooling_time':118, 'fluid':{'amount':200,'tag':'tconstruct:molten_diamond'},
            'result':'tconstruct:jeweled_apple'})
        self.assertEqual(gem['result'], {'amount':100, 'fluid':'tconstruct:molten_diamond'})
        self.assertEqual(gem['temperature'], 1450)
        declaration = json.loads((ROOT / 'content/data/tconstruct/horizon/item/jeweled_apple.json').read_text())
        food = declaration['components']['minecraft:food']
        self.assertEqual(food, {'nutrition':4, 'saturation':9.6, 'can_always_eat':True})
        consume = declaration['components']['minecraft:consumable']
        self.assertEqual(consume['on_consume_effects'][0]['effects'], [
            {'id':'minecraft:haste','duration':1200}, {'id':'minecraft:resistance','duration':2400}])
        texture = (ROOT / 'content/assets/tconstruct/textures/item/food/jeweled_apple.png').read_bytes()
        self.assertEqual(hashlib.sha1(b'blob ' + str(len(texture)).encode() + b'\0' + texture).hexdigest(), 'abb3157e5ea06ffcc67fbeef3d6ffa7064e5e37a')
        with zipfile.ZipFile(ROOT / 'content/reference/tconstruct-fluid-tags.zip') as z:
            tag = json.loads(z.read('data/tconstruct/tags/fluids/molten_diamond.json'))
        self.assertIn('tconstruct:molten_diamond', tag['values'])

if __name__ == '__main__': unittest.main()
