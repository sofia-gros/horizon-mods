"""Cheese Block registration; standalone content stays non-edible."""
import hashlib,json,sys,tempfile,unittest,zipfile
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
sys.path.insert(0,str(ROOT/'tools'))
from package_hmod import package

def blob(data):return hashlib.sha1(f'blob {len(data)}\0'.encode()+data).hexdigest()
class CheeseBlock(unittest.TestCase):
 def test_registration_assets_and_conversions(self):
  decl=json.loads((ROOT/'content/data/tconstruct/horizon/block/cheese_block.json').read_text())
  self.assertEqual(decl['template'],'minecraft:glass')
  self.assertNotIn('minecraft:food',decl.get('components',{})); self.assertNotIn('minecraft:consumable',decl.get('components',{}))
  self.assertEqual(decl['properties'],{'hardness':1.5,'explosion_resistance':3.0,'requires_tool':False,'sounds':'minecraft:honey_block','map_color':18})
  for path,sha in [('blockstates/cheese_block.json','b561dc7ba02c77149a67bf013490f71b5ce601ee'),('models/block/storage/cheese.json','d60da4bdc72535a9ddaf2d5f2fa66968ffcbfc36'),('textures/block/storage/cheese_block_inner.png','7c655f5c77b13928c576c03889684a1c70104cc2'),('textures/block/storage/cheese_block_outer.png','e2ade6ada593ea5898278357889e50dd98b6641b')]:
   self.assertEqual(blob((ROOT/'content/assets/tconstruct'/path).read_bytes()),sha)
  recipes=ROOT/'content/data/tconstruct/recipe'
  a=json.loads((recipes/'cheese_block_from_ingot.json').read_text());b=json.loads((recipes/'cheese_ingot_from_block.json').read_text())
  self.assertEqual((a['pattern'],a['key'],a['result']),(['##','##'],{'#':'tconstruct:cheese_ingot'},{'id':'tconstruct:cheese_block'}))
  self.assertEqual((b['ingredients'],b['result']),(['tconstruct:cheese_block'],{'id':'tconstruct:cheese_ingot','count':4}))
  with zipfile.ZipFile(ROOT/'content/reference/tconstruct-recipes.zip') as z:
   basin=json.loads(z.read('data/tconstruct/recipes/smeltery/casting/cheese_block.json'))
   self.assertEqual((basin['result'],basin['fluid'],basin['cooling_time']),('tconstruct:cheese_block',{'amount':1000,'tag':'forge:milk'},6000))
  tag=json.loads((ROOT/'content/data/minecraft/tags/block/mineable/shovel.json').read_text()); self.assertIn('tconstruct:cheese_block',tag['values'])
  src=(ROOT/'mixins/tconstruct/resource-tables.rs').read_text(); self.assertIn('apply_cheese_movement(&mut named.tag)',src); self.assertIn('apply_cheese_behavior(&mut behaviors)',src); self.assertIn('apply_cheese_lighting(row)',src); self.assertIn('tconstruct-native-creative-v8',src); self.assertIn('"block-behaviours.json"',src)
 def test_package_has_registered_block_and_recipes(self):
  with tempfile.TemporaryDirectory() as tmp:
   out=Path(tmp)/'mod.hmod';package(out)
   with zipfile.ZipFile(out) as z:
    for path in ('data/tconstruct/horizon/block/cheese_block.json','data/tconstruct/loot_table/blocks/cheese_block.json','data/tconstruct/recipe/cheese_block_from_ingot.json','data/tconstruct/recipe/cheese_ingot_from_block.json','assets/tconstruct/blockstates/cheese_block.json','assets/tconstruct/models/block/storage/cheese.json'):
     self.assertEqual(z.read(path),(ROOT/'content'/path).read_bytes())
if __name__=='__main__':unittest.main()
