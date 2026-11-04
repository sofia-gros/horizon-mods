from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1] / 'mixins/tconstruct'

class FlowingVanillaDisplay(unittest.TestCase):
    def test_display_alias_is_wired_without_changing_tank_identity(self):
        render = (ROOT / 'source-melter-render.rs').read_text()
        gauge = (ROOT / 'source-gauge.rs').read_text()
        manifest = (ROOT / 'horizon-mixin.toml').read_text()
        self.assertIn('tconstruct_source_fluid_display::sprite(fluid)', render)
        self.assertIn('let fluid = super::tconstruct_source_fluid_display::display_id(fluid);', render)
        self.assertIn('match super::tconstruct_source_fluid_display::display_id(fluid)', render)
        self.assertIn('let display_fluid = super::tconstruct_source_fluid_display::display_id(&fluid);', gauge)
        self.assertIn('if fluid == "tconstruct:potion"', gauge)
        self.assertIn('source = "source-fluid-display.rs"', manifest)
        for file in ('source-fluid-container.rs', 'source-smeltery-tank.rs'):
            self.assertNotIn('tconstruct_source_fluid_display', (ROOT / file).read_text())

if __name__ == '__main__':
    unittest.main()
