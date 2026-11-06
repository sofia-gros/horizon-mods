"""Only actual fuel/catalog consumers add the shared potion adapter."""
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[2]


class PotionFixtureWiring(unittest.TestCase):
    def test_every_direct_fuel_fixture_uses_the_actual_adapter(self):
        fixtures = ROOT / 'tconstruct/tests/rust'
        consumers = [p for p in fixtures.glob('*.rs') if 'source-smeltery-fuel.rs' in p.read_text()]
        self.assertEqual(len(consumers), 8)
        for path in consumers:
            self.assertIn('pluto_potion_fixture.rs', path.read_text(), str(path))
            self.assertIn('pluto_storage_fixture.rs', path.read_text(), str(path))
        helper = (fixtures / 'pluto_potion_fixture.rs').read_text()
        self.assertIn('../../../pluto-lib/mixins/pluto_native_creative/potion-facts.rs', helper)
        self.assertNotIn('extern crate self', helper)
        self.assertNotIn('fn potion(', helper)

    def test_generic_storage_projection_dependencies_are_unchanged(self):
        fixtures = ROOT / 'tconstruct/tests/rust'
        self.assertNotIn('pluto_potions', (fixtures / 'pluto_storage_fixture.rs').read_text())
        runner = (ROOT / 'tconstruct/tests/run_inventory_projection.py').read_text()
        self.assertNotIn('horizon-client-core', runner)


if __name__ == '__main__':
    unittest.main()
