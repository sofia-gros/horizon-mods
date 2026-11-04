"""Source and registration checks for the Silky Cloth casting result."""
import hashlib, json, unittest, zipfile
from pathlib import Path
ROOT = Path(__file__).resolve().parents[1]
class SilkyCloth(unittest.TestCase):
 def test_pinned_recipes_and_item(self):
  with zipfile.ZipFile(ROOT / 'content/reference/tconstruct-recipes.zip') as z:
   casting=json.loads(z.read('data/tconstruct/recipes/tools/modifiers/silky_cloth.json'))
   melting=json.loads(z.read('data/tconstruct/recipes/smeltery/melting/metal/rose_gold/silky_cloth.json'))
  self.assertEqual(casting,json.loads((ROOT/'tests/fixtures/silky_cloth_casting.json').read_text()))
  self.assertEqual(melting,json.loads((ROOT/'tests/fixtures/silky_cloth_melting.json').read_text()))
  self.assertEqual((casting['cast'],casting['cast_consumed'],casting['fluid'],casting['cooling_time'],casting['result']),({'item':'minecraft:cobweb'},True,{'amount':90,'tag':'forge:molten_rose_gold'},52,'tconstruct:silky_cloth'))
  self.assertEqual((melting['ingredient'],melting['result'],melting['temperature'],melting['time']),({'item':'tconstruct:silky_cloth'},{'amount':90,'tag':'forge:molten_rose_gold'},550,52))
  self.assertEqual(json.loads((ROOT/'content/data/tconstruct/horizon/item/silky_cloth.json').read_text())['template'],'minecraft:flint')
  self.assertEqual(hashlib.sha256((ROOT/'content/assets/tconstruct/textures/item/materials/silky_cloth.png').read_bytes()).hexdigest(),'2f36aa79c7791cec7e5b88aa19f9fb39401f1ef61517e04a7cf4bbf948560970')
if __name__ == '__main__': unittest.main()
