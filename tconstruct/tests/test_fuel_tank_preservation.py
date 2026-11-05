from pathlib import Path
import unittest
ROOT = Path(__file__).resolve().parents[1]

class FuelTankPreservation(unittest.TestCase):
    def test_guard_precedes_commit_and_inventory_changes(self):
        source = (ROOT / 'mixins/tconstruct/source-machine-runtime.rs').read_text()
        refresh = source.split('pub(super) fn refresh_smeltery',1)[1].split('pub(super) fn interact_fuel_tank',1)[0]
        self.assertLess(refresh.index('tank.to_nbt_preserving_fuel(&raw_tank)'), refresh.index('state.fuel = fuel.duration'))
        self.assertIn('fuel_update = Some((at, kind, updated_tank))', refresh)
        interaction = source.split('pub(super) fn interact_fuel_tank',1)[1].split('\npub(super) fn ',1)[0]
        self.assertLess(interaction.index('tank.to_nbt_preserving_fuel(&raw_tank)'), interaction.index('write_machine_updates(game,'))
        self.assertLess(interaction.index('write_machine_updates(game,'), interaction.index('game.held_mut()'))
        self.assertIn('else { return true; }', interaction)

if __name__ == '__main__': unittest.main()
