"""Pinned source, shared module ownership and bounded acquisition wiring."""
import json
from pathlib import Path
import tomllib
import unittest
from zipfile import ZipFile
ROOT=Path(__file__).resolve().parents[1]
MIX=ROOT/'mixins/tconstruct'
class ExpandedHammer(unittest.TestCase):
 def test_source_fixtures_are_exact(self):
  for pack,path,fixture in [('recipes','recipes/tools/modifiers/ability/expanded','expanded_recipe'),('modifiers','tinkering/modifiers/expanded','expanded_modifier'),('tool-definitions','tinkering/tool_definitions/sledge_hammer','expanded_hammer')]:
   with ZipFile(ROOT/f'content/reference/tconstruct-{pack}.zip') as z:self.assertEqual(z.read(f'data/tconstruct/{path}.json'),(ROOT/f'tests/fixtures/{fixture}.json').read_bytes())
 def test_shared_station_dependency_exists_in_both_crates(self):
  modules=tomllib.loads((MIX/'horizon-mixin.toml').read_text())['module']
  station={m['file'] for m in modules if m.get('source')=='source-station-world.rs'}
  expanded={m['file'] for m in modules if m.get('source')=='source-expanded.rs'}
  self.assertEqual(station,expanded)
  self.assertNotIn('tconstruct_modifier_snapshot::',(MIX/'source-expanded.rs').read_text())
 def test_live_gate_before_mutation_and_ci(self):
  station=(MIX/'source-station-world.rs').read_text();start=station.index('fn upgrade_plan(');s=station[start:]
  self.assertLess(s.index('tconstruct_expanded::level(world,tool)?'),s.index('prepare_for_tool_mutation(tool)'))
  self.assertIn('source-area-protection.rs',(ROOT/'tests/rust/source_area_protection.rs').read_text())
  workflow=(ROOT.parent/'.github/workflows/mixins.yml').read_text()
  self.assertIn('python3 tconstruct/tests/run_expanded_hammer.py --horizon ../horizon-official',workflow)
  self.assertIn('tconstruct/tests/rust/source_area_protection.rs',workflow)
