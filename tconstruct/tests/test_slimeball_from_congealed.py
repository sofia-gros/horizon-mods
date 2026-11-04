"""Pinned direct-input congealed slime unpacking and native package."""
import json,sys,tempfile,unittest
from pathlib import Path
from zipfile import ZipFile
ROOT=Path(__file__).resolve().parents[1]
DATA=ROOT/'content/data/tconstruct'
sys.path.insert(0,str(ROOT/'tools'))
from package_hmod import package
KINDS=('earth','sky','ichor','ender')
class SlimeballFromCongealed(unittest.TestCase):
 def test_source_and_native_recipes(self):
  with ZipFile(ROOT/'content/reference/tconstruct-recipes.zip') as z:
   for kind in KINDS:
    name=f'{kind}_slimeball_from_congealed'; ingredient=f'tconstruct:{kind}_congealed_slime'; result='minecraft:slime_ball' if kind=='earth' else f'tconstruct:{kind}_slime_ball'
    source=json.loads(z.read(f'data/tconstruct/recipes/common/slime/{kind}/slimeball_from_congealed.json'))
    self.assertEqual(source['ingredients'],[{'item':ingredient}]); self.assertEqual(source['result'],{'count':4,'item':result})
    expected={'type':source['type'],'category':source['category'],'group':source['group'],'ingredients':[ingredient],'result':{'count':4,'id':result}}
    self.assertEqual(json.loads((DATA/'recipe'/f'{name}.json').read_text()),expected)
    self.assertTrue((DATA/'horizon/block'/f'{kind}_congealed_slime.json').is_file())
 def test_recipe_unlocks(self):
  for kind in KINDS:
   name=f'{kind}_slimeball_from_congealed'; ingredient=f'tconstruct:{kind}_congealed_slime'
   adv=json.loads((DATA/'advancement/recipes/misc'/f'{name}.json').read_text())
   self.assertEqual(adv['parent'],'minecraft:recipes/root')
   self.assertEqual(adv['criteria']['has_item'],{'trigger':'minecraft:inventory_changed','conditions':{'items':[{'items':[ingredient]}]}})
   self.assertEqual(adv['criteria']['has_the_recipe'],{'trigger':'minecraft:recipe_unlocked','conditions':{'recipes':f'tconstruct:{name}'}})
   self.assertEqual(adv['requirements'],[['has_item','has_the_recipe']]); self.assertEqual(adv['rewards']['recipes'],[f'tconstruct:{name}'])
 def test_package_contains_recipes_unlocks_and_results(self):
  with tempfile.TemporaryDirectory() as tmp:
   out=Path(tmp)/'mod.hmod';package(out)
   with ZipFile(out) as z:
    for kind in KINDS:
     name=f'{kind}_slimeball_from_congealed'
     for prefix in ('data/tconstruct/recipe','data/tconstruct/advancement/recipes/misc'):
      path=f'{prefix}/{name}.json';self.assertEqual(json.loads(z.read(path)),json.loads((ROOT/'content'/path).read_text()))
     if kind!='earth':self.assertIn(f'data/tconstruct/horizon/item/{kind}_slime_ball.json',z.namelist())
if __name__=='__main__': unittest.main()
