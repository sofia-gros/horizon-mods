"""Pinned slime fern loot conditions; only the nether ferns drop without shears."""
import json, unittest
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
class SlimeFernLoot(unittest.TestCase):
 def test_all_source_variants(self):
  for color in ('earth','sky','ender','ichor','blood'):
   for kind in ('fern','tall_grass'):
    name=f'{color}_slime_{kind}'
    with self.subTest(name=name):
     value=json.loads((ROOT/f'content/data/tconstruct/loot_table/blocks/{name}.json').read_text())
     expected={'type':'minecraft:survives_explosion'} if color in ('ichor','blood') and kind=='fern' else 'minecraft:tool/can_shear'
     self.assertEqual(value['random_sequence'],f'tconstruct:blocks/{name}')
     self.assertEqual(value['pools'],[{'condition':expected,'entries':[{'type':'minecraft:item','name':f'tconstruct:{name}'}],'rolls':1}])
     self.assertTrue((ROOT/f'content/data/tconstruct/horizon/block/{name}.json').is_file())
     if color in ('ichor','blood') and kind == 'fern':
      source=json.loads((ROOT/f'tests/fixtures/slime-fern-source/{name}.json').read_text())
      self.assertEqual(source['pools'][0]['conditions'], [{'condition':'minecraft:survives_explosion'}])
      self.assertEqual(source['pools'][0]['entries'], [{'type':'minecraft:item','name':f'tconstruct:{name}'}])
if __name__=='__main__': unittest.main()
