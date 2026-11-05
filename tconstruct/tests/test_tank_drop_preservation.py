from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]


class TankDropPreservation(unittest.TestCase):
    def test_remove_and_place_transport_raw_tank_instead_of_reserializing(self):
        remove = (ROOT / 'mixins/tconstruct/source-machine-remove.rs').read_text()
        remove = remove.split('if let Some(capacity)', 1)[1].split('if matches!(&*name, "tconstruct:smeltery_controller"', 1)[0]
        self.assertIn('copy_native_tank(&raw, capacity)', remove)
        self.assertIn('saved.get("tank")?.clone()', remove)
        self.assertIn('Tank::new(capacity).to_nbt()', remove)
        self.assertIn('insert("tconstruct:internal_tank_copy".into(), tank)', remove)
        self.assertNotIn('tank.to_nbt()', remove)
        place = (ROOT / 'mixins/tconstruct/source-machine-place.rs').read_text().split('let Some(capacity)', 1)[1]
        self.assertIn('copy_native_tank(saved, capacity)', place)
        self.assertIn('fields.insert("tank".into(), tank)', place)
        self.assertNotIn('tank.to_nbt()', place)
        self.assertNotIn('state.tank = tank', place)

    def test_drop_fixture_extracts_production_marker_transport(self):
        runner = (ROOT / 'tests/run_drain_container.py').read_text()
        self.assertIn('mixins/tconstruct/source-smeltery-tank-drop.rs', runner)
        self.assertIn('replace("// __PRODUCTION_TANK_DROP__", tank_drop)', runner)
        self.assertIn('portable_tank_drop_keeps_raw_payload_for_all_twelve_items', runner)


if __name__ == '__main__':
    unittest.main()
