"""Partial World Creative input and native rebuild registration."""
import json, sys, tempfile, unittest, zipfile
from pathlib import Path
MOD=Path(__file__).resolve().parents[1]
sys.path.insert(0,str(MOD/'tools'))
from package_hmod import package
class WorldCreativeTab(unittest.TestCase):
 def test_packaged_world_tab_and_rebuild(self):
  with tempfile.TemporaryDirectory() as tmp:
   path=Path(tmp)/'mod.hmod'; package(path)
   with zipfile.ZipFile(path) as z:
    names=[n for n in z.namelist() if n.endswith('/creative-tabs.json')]
    self.assertEqual(len(names),1)
    groups=json.loads(z.read(names[0])); world=[g for g in groups if g['id']=='tconstruct:world']
    self.assertEqual(len(world),1)
    self.assertEqual(world[0]['icon'],'{count:1,id:"tconstruct:cobalt_ore"}')
    self.assertEqual(world[0]['items'],['{count:1,id:"tconstruct:necrotic_bone"}'])
    self.assertIn('data/tconstruct/horizon/item/necrotic_bone.json',z.namelist())
  src=(MOD/'mixins/tconstruct/resource-tables.rs').read_text()
  self.assertIn('Some("tconstruct:tools" | "tconstruct:blocks" | "tconstruct:world")',src)
if __name__ == '__main__': unittest.main()
