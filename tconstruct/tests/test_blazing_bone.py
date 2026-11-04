"""Pinned Blazing Bone registration and casting input."""
import hashlib, json, unittest, zipfile
from pathlib import Path
ROOT = Path(__file__).resolve().parents[1]
class BlazingBone(unittest.TestCase):
 def test_source_recipe_and_item(self):
  with zipfile.ZipFile(ROOT / 'content/reference/tconstruct-recipes.zip') as z:
   src=json.loads(z.read('data/tconstruct/recipes/smeltery/casting/blaze/bone.json'))
  self.assertEqual(src,json.loads((ROOT / 'tests/fixtures/blazing_bone.json').read_text()))
  self.assertEqual((src['cast'],src['cast_consumed'],src['fluid'],src['cooling_time'],src['result']),({'tag':'forge:wither_bones'},True,{'amount':200,'tag':'tconstruct:blazing_blood'},120,'tconstruct:blazing_bone'))
  self.assertEqual(json.loads((ROOT/'content/data/tconstruct/horizon/item/blazing_bone.json').read_text())['template'],'minecraft:bone')
  p=ROOT/'content/assets/tconstruct/textures/item/materials/blazing_bone.png'
  self.assertEqual(hashlib.sha256(p.read_bytes()).hexdigest(),'8d7f5f4572ad38fac454071e0f95ac9febc84d9a962357a4331817c0ac73dd13')
if __name__ == '__main__': unittest.main()
