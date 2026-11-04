"""Necrotic Bone registration, source texture and bundled tag membership."""
import hashlib, json, sys, tempfile, unittest, zipfile
from pathlib import Path
ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / 'tools'))
from package_hmod import package
class NecroticBone(unittest.TestCase):
 def test_source_texture_and_tags(self):
  image = ROOT / 'content/assets/tconstruct/textures/item/materials/necrotic_bone.png'
  self.assertEqual(hashlib.sha256(image.read_bytes()).hexdigest(), 'a9affb6eedb438ba54de7b7d87f2d9d9cec2ad99c2c076cea3a3e4be8bd6b611')
  tags = ROOT / 'content/reference/tconstruct-tags/data/forge/tags/items'
  self.assertEqual(json.loads((tags/'wither_bones.json').read_text())['values'], ['tconstruct:necrotic_bone', '#forge:bones/wither'])
  self.assertEqual(json.loads((tags/'bones/wither.json').read_text())['values'], ['tconstruct:necrotic_bone'])
 def test_package_has_item_and_assets(self):
  with tempfile.TemporaryDirectory() as tmp:
   out=Path(tmp)/'test.hmod'; package(out)
   with zipfile.ZipFile(out) as z:
    self.assertEqual(json.loads(z.read('data/tconstruct/horizon/item/necrotic_bone.json')), {'template':'minecraft:bone'})
    self.assertEqual(json.loads(z.read('assets/tconstruct/models/item/necrotic_bone.json'))['textures']['layer0'], 'tconstruct:item/materials/necrotic_bone')
    self.assertIn('assets/tconstruct/items/necrotic_bone.json', z.namelist())
    self.assertEqual(hashlib.sha256(z.read('assets/tconstruct/textures/item/materials/necrotic_bone.png')).hexdigest(), 'a9affb6eedb438ba54de7b7d87f2d9d9cec2ad99c2c076cea3a3e4be8bd6b611')
if __name__ == '__main__': unittest.main()
