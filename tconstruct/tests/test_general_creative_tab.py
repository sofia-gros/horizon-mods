"""Partial General Creative input and native rebuild registration."""
import json, sys, tempfile, unittest, zipfile
from pathlib import Path
MOD=Path(__file__).resolve().parents[1]
sys.path.insert(0,str(MOD/'tools'))
from package_hmod import package
class GeneralCreativeTab(unittest.TestCase):
 def test_packaged_general_tab_and_rebuild(self):
  with tempfile.TemporaryDirectory() as tmp:
   path=Path(tmp)/'mod.hmod'; package(path)
   with zipfile.ZipFile(path) as z:
    names=[n for n in z.namelist() if n.endswith('/creative-tabs.json')]
    self.assertEqual(len(names),1)
    groups=json.loads(z.read(names[0])); general=[g for g in groups if g['id']=='tconstruct:general']
    self.assertEqual(len(general),1)
    self.assertEqual(general[0]['title'], "Tinkers' General")
    self.assertEqual(general[0]['icon'],'{count:1,id:"tconstruct:materials_and_you"}')
    expected=['blazing_bone','silky_cloth','seared_reinforcement','bacon', 'materials_and_you','puny_smelting','mighty_smelting','encyclopedia', 'clear_glass','clear_tinted_glass','soul_glass','clear_glass_pane','soul_glass_pane','gold_bars','obsidian_pane']
    self.assertEqual(general[0]['items'],[f'{{count:1,id:"tconstruct:{n}"}}' for n in expected])
    for n in expected[:8]: self.assertIn(f'data/tconstruct/horizon/item/{n}.json',z.namelist())
    for n in expected[8:]:
     name=f'data/tconstruct/horizon/block/{n}.json'
     self.assertIn(name,z.namelist())
     self.assertTrue(json.loads(z.read(name)).get('item',True))
  src=(MOD/'mixins/tconstruct/resource-tables.rs').read_text()
  self.assertIn('Some("tconstruct:tools" | "tconstruct:blocks" | "tconstruct:world" | "tconstruct:general")',src)
if __name__ == '__main__': unittest.main()
