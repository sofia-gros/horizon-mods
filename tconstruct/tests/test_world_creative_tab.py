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
    expected=['cobalt_ore','raw_cobalt','raw_cobalt_block','necrotic_bone',
              'earth_congealed_slime','sky_slime_ball','sky_congealed_slime',
              'ichor_slime_ball','ichor_congealed_slime','ender_slime_ball','ender_congealed_slime',
              'earth_slime_sapling','earth_slime_leaves','sky_slime_sapling','sky_slime_leaves',
              'blood_slime_sapling','blood_slime_leaves','ender_slime_sapling','ender_slime_leaves',
              'earth_slime_grass_seeds','sky_slime_grass_seeds','blood_slime_grass_seeds','ender_slime_grass_seeds']
    self.assertEqual(world[0]['items'],[f'{{count:1,id:"tconstruct:{n}"}}' for n in expected])
    for n in expected:
     self.assertTrue(any(f'data/tconstruct/horizon/{kind}/{n}.json' in z.namelist() for kind in ('block','item')), n)
    self.assertIn('data/tconstruct/horizon/item/necrotic_bone.json',z.namelist())
  src=(MOD/'mixins/tconstruct/resource-tables.rs').read_text()
  self.assertIn('Some("tconstruct:tools" | "tconstruct:blocks" | "tconstruct:world" | "tconstruct:general")',src)
if __name__ == '__main__': unittest.main()
