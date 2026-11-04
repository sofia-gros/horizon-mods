"""Pinned source direct-input furnace recipes and native package adaptation."""
import json, sys, tempfile, unittest
from pathlib import Path
from zipfile import ZipFile
ROOT=Path(__file__).resolve().parents[1]
DATA=ROOT/'content/data/tconstruct'
sys.path.insert(0,str(ROOT/'tools'))
from package_hmod import package
CASES=[('seared','seared_stone_smelting','seared_cobble','seared_stone'),
       ('seared','seared_paver_smelting','seared_stone','seared_paver'),
       ('seared','seared_cracked_bricks_smelting','seared_bricks','seared_cracked_bricks'),
       ('scorched','scorched_road_smelting','scorched_stone','scorched_road')]
class SmelteryStoneFurnace(unittest.TestCase):
 def test_original_and_native_recipes(self):
  with ZipFile(ROOT/'content/reference/tconstruct-recipes.zip') as z:
   for kind,name,ingredient,result in CASES:
    source=json.loads(z.read(f'data/tconstruct/recipes/smeltery/{kind}/{name}.json'))
    self.assertEqual(source['type'],'minecraft:smelting')
    self.assertEqual(source['ingredient'],{'item':f'tconstruct:{ingredient}'})
    self.assertEqual(source['result'],f'tconstruct:{result}')
    expected={'type':'minecraft:smelting','category':source['category'],'cookingtime':source['cookingtime'],
              'experience':source['experience'],'ingredient':f'tconstruct:{ingredient}',
              'result':{'id':f'tconstruct:{result}'}}
    self.assertEqual(json.loads((DATA/'recipe'/f'{name}.json').read_text()),expected)
    for block in (ingredient,result): self.assertTrue((DATA/'horizon/block'/f'{block}.json').is_file())
 def test_recipe_unlocks_use_adapted_ids(self):
  for _,name,ingredient,_ in CASES:
   adv=json.loads((DATA/'advancement/recipes/building_blocks'/f'{name}.json').read_text())
   self.assertEqual(adv['parent'],'minecraft:recipes/root')
   self.assertEqual(adv['criteria']['has_item'],{'trigger':'minecraft:inventory_changed','conditions':{'items':[{'items':[f'tconstruct:{ingredient}']}]}})
   self.assertEqual(adv['criteria']['has_the_recipe'],{'trigger':'minecraft:recipe_unlocked','conditions':{'recipes':f'tconstruct:{name}'}})
   self.assertEqual(adv['requirements'],[['has_item','has_the_recipe']])
   self.assertEqual(adv['rewards']['recipes'],[f'tconstruct:{name}'])
 def test_package_has_recipes_and_unlocks(self):
  with tempfile.TemporaryDirectory() as tmp:
   out=Path(tmp)/'mod.hmod'; package(out)
   with ZipFile(out) as z:
    for _,name,_,_ in CASES:
     for prefix in ('data/tconstruct/recipe','data/tconstruct/advancement/recipes/building_blocks'):
      path=f'{prefix}/{name}.json'; self.assertEqual(json.loads(z.read(path)),json.loads((ROOT/'content'/path).read_text()))
if __name__=='__main__': unittest.main()
