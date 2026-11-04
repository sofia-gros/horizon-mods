"""Pinned four-stone to four-bricks recipes and existing alternative."""
import json,sys,tempfile,unittest
from pathlib import Path
from zipfile import ZipFile
ROOT=Path(__file__).resolve().parents[1]; DATA=ROOT/'content/data/tconstruct'
sys.path.insert(0,str(ROOT/'tools'))
from package_hmod import package
CASES=(('seared','seared_stone'),('scorched','polished_scorched_stone'))
class StoneToBricks(unittest.TestCase):
 def test_source_and_existing_alternative(self):
  with ZipFile(ROOT/'content/reference/tconstruct-recipes.zip') as z:
   for kind,ingredient in CASES:
    name=f'{kind}_bricks_crafting'; src=json.loads(z.read(f'data/tconstruct/recipes/smeltery/{kind}/{name}.json'))
    self.assertEqual(src['key'],{'b':{'item':f'tconstruct:{ingredient}'}}); self.assertEqual(src['pattern'],['bb','bb'])
    self.assertEqual(src['result'],{'count':4,'item':f'tconstruct:{kind}_bricks'})
    actual=json.loads((DATA/'recipe'/f'{name}.json').read_text()); self.assertEqual(actual,{'type':src['type'],'category':src['category'],'key':{'b':f'tconstruct:{ingredient}'},'pattern':src['pattern'],'result':{'count':4,'id':f'tconstruct:{kind}_bricks'}})
    old=json.loads((DATA/'recipe'/f'survival_{kind}_{kind}_bricks_from_brick.json').read_text())
    self.assertEqual(old['key']['b'],f'tconstruct:{kind}_brick'); self.assertEqual(old['result']['id'],f'tconstruct:{kind}_bricks'); self.assertEqual(old['result'].get('count',1),1)
    for block in (ingredient,f'{kind}_bricks'): self.assertTrue((DATA/'horizon/block'/f'{block}.json').is_file())
 def test_recipe_unlocks(self):
  for kind,ingredient in CASES:
   name=f'{kind}_bricks_crafting'; adv=json.loads((DATA/'advancement/recipes/building_blocks'/f'{name}.json').read_text())
   self.assertEqual(adv['criteria']['has_item'],{'trigger':'minecraft:inventory_changed','conditions':{'items':[{'items':[f'tconstruct:{ingredient}']}]}})
   self.assertEqual(adv['criteria']['has_the_recipe'],{'trigger':'minecraft:recipe_unlocked','conditions':{'recipes':f'tconstruct:{name}'}})
   self.assertEqual(adv['requirements'],[['has_item','has_the_recipe']]); self.assertEqual(adv['rewards']['recipes'],[f'tconstruct:{name}'])
 def test_package_contains_recipes_and_unlocks(self):
  with tempfile.TemporaryDirectory() as tmp:
   out=Path(tmp)/'mod.hmod';package(out)
   with ZipFile(out) as z:
    for kind,_ in CASES:
     name=f'{kind}_bricks_crafting'
     for prefix in ('data/tconstruct/recipe','data/tconstruct/advancement/recipes/building_blocks'):
      path=f'{prefix}/{name}.json'; self.assertEqual(json.loads(z.read(path)),json.loads((ROOT/'content'/path).read_text()))
if __name__=='__main__': unittest.main()
