"""Pin the source acquisition chain and the registered pane assets."""
import json
from pathlib import Path
import unittest
import zipfile
ROOT=Path(__file__).resolve().parents[1]
class ReinforcedRecipes(unittest.TestCase):
 def test_source_acquisition_and_upgrade(self):
  with zipfile.ZipFile(ROOT/'content/reference/tconstruct-recipes.zip') as z:
   def recipe(path):return json.loads(z.read('data/tconstruct/recipes/'+path+'.json'))
   obsidian=recipe('smeltery/melting/obsidian/block');emerald=recipe('smeltery/melting/emerald/gem')
   pane=recipe('smeltery/casting/obsidian/pane');reinforcement=recipe('tools/modifiers/emerald_reinforcement');upgrade=recipe('tools/modifiers/upgrade/reinforced')
   self.assertEqual((obsidian['result']['fluid'],obsidian['result']['amount']),('tconstruct:molten_obsidian',1000))
   self.assertEqual((emerald['result']['fluid'],emerald['result']['amount']),('tconstruct:molten_emerald',100))
   self.assertEqual((pane['result'],pane['fluid']['amount']),('tconstruct:obsidian_pane',250))
   self.assertEqual((reinforcement['result'],reinforcement['cast']['item'],reinforcement['cast_consumed'],reinforcement['fluid']['amount']),('tconstruct:emerald_reinforcement','tconstruct:obsidian_pane',True,25))
   self.assertEqual((upgrade['type'],upgrade['needed_per_level'],upgrade['level']['max'],upgrade['slots']['upgrades'],upgrade['input']['item']),('tconstruct:incremental_modifier',4,5,1,'tconstruct:emerald_reinforcement'))
  bypass=json.loads((ROOT/'content/reference/tconstruct-tags/data/tconstruct/tinkering/tags/modifiers/durability/bypass_reinforced.json').read_text())
  self.assertEqual(bypass['values'],['tconstruct:glowing'])
  declaration=json.loads((ROOT/'content/data/tconstruct/horizon/block/obsidian_pane.json').read_text())
  self.assertEqual((declaration['template'],declaration['properties']['hardness'],declaration['properties']['requires_tool']),('minecraft:glass_pane',25.0,True))
  self.assertTrue((ROOT/'assets/tconstruct/textures/item/materials/reinforcement/emerald.png').is_file())
if __name__=='__main__':unittest.main()
