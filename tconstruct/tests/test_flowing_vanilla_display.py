from pathlib import Path
import json
import re
import unittest

ROOT = Path(__file__).resolve().parents[1] / 'mixins/tconstruct'

class FlowingVanillaDisplay(unittest.TestCase):
    def test_tconstruct_aliases_match_pinned_fluid_inventory(self):
        fluids = json.loads((ROOT.parents[1] / 'content/reference/tconstruct-melter-fluids.json').read_text())
        pairs = dict(re.findall(r'"(tconstruct:flowing_[^"]+)" => "(tconstruct:[^"]+)"', (ROOT / 'source-fluid-display.rs').read_text()))
        flowing = {fluid for fluid in fluids if fluid.startswith('tconstruct:flowing_')}
        self.assertEqual(len(flowing), 70)
        self.assertEqual(set(pairs), flowing)
        for fluid, still in pairs.items():
            self.assertEqual(still, fluid.replace('flowing_', '', 1))
            self.assertIn(still, fluids)

    def test_display_alias_is_wired_without_changing_tank_identity(self):
        render = (ROOT / 'source-melter-render.rs').read_text()
        gauge = (ROOT / 'source-gauge.rs').read_text()
        manifest = (ROOT / 'horizon-mixin.toml').read_text()
        self.assertIn('tconstruct_source_fluid_display::sprite(fluid)', render)
        self.assertIn('let fluid = super::tconstruct_source_fluid_display::display_id(fluid);', render)
        self.assertEqual(render.count('match super::tconstruct_source_fluid_display::display_id(fluid)'), 2)
        self.assertIn('let display_fluid = super::tconstruct_source_fluid_display::display_id(&fluid);', gauge)
        self.assertIn('if fluid == "tconstruct:potion"', gauge)
        self.assertIn('source = "source-fluid-display.rs"', manifest)
        for file in ('source-fluid-container.rs', 'source-smeltery-tank.rs'):
            self.assertNotIn('tconstruct_source_fluid_display', (ROOT / file).read_text())

if __name__ == '__main__':
    unittest.main()
