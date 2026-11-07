"""Display wiring checks; production state behavior runs in the Rust fixture."""
from pathlib import Path
import unittest
ROOT = Path(__file__).resolve().parents[1]

class HeaterTooltip(unittest.TestCase):
    def test_uses_persisted_temperature_without_inventing_queued_heat(self):
        text = (ROOT / 'mixins/tconstruct/station-result-tooltip.rs').read_text()
        melter = text.split('if crate::game::tconstruct_ui::is_melter(game)', 1)[1].split('if crate::game::tconstruct_source_smeltery_ui::is_smeltery(game)', 1)[0]
        self.assertIn('state.solid_fuel_tooltip_temperature()', melter)
        for forbidden in ['queued_temperature', 'solid_fuel_rule(', 'burn_time(', 'state.fuel > 0']:
            self.assertNotIn(forbidden, melter)
        self.assertIn('Melter::from_nbt(&tag)', melter)
        self.assertIn('__horizon_mixins_tconstruct_remote_screen_position()', melter)
        self.assertIn('remote_chest_menu()', melter)
        self.assertIn('(!has_heater || game.cursor.1 > oy + 50.0)', melter)
        self.assertIn('else if let Some(temperature) = temperature', melter)
        self.assertIn('gui.tconstruct.melting.fuel.empty', melter)
