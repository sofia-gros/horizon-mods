from pathlib import Path
import unittest
ROOT=Path(__file__).resolve().parents[1]
class TankComparator(unittest.TestCase):
    def test_registered_tanks_share_capacity_and_native_signal_routes(self):
        p=ROOT/'mixins/tconstruct'
        capacities=(p/'source-smeltery-fuel.rs').read_text().split('/// Portable tank items')[0]
        for metal in ('seared','scorched'):
            for kind in ('fuel_tank','fuel_gauge','ingot_tank','ingot_gauge'):
                self.assertIn(f'tconstruct:{metal}_{kind}',capacities)
        self.assertIn('tconstruct_source_smeltery_fuel::capacity', (p/'source-native-hook-bodies.rs').read_text().split('pub(super) fn tconstruct_geode_random_tick')[0])
        self.assertIn('saved_comparator(key_name, &raw)',(p/'source-casting-comparator.rs').read_text())
    def test_fuel_burn_held_and_placement_commit_before_signal(self):
        p=ROOT/'mixins/tconstruct'
        runtime=(p/'source-machine-runtime.rs').read_text()
        for start,end in [('pub(super) fn refresh_smeltery(', 'pub(super) fn interact_fuel_tank('),('pub(super) fn tick_melter(', 'pub(super) fn casting_at('),('fn smeltery_to_tank(', 'fn tank_casting_recipe_current(')]:
            part=runtime.split(start,1)[1].split(end,1)[0]
            self.assertIn('write_machine_updates(game,',part)
        held=runtime.split('pub(super) fn interact_fuel_tank(',1)[1].split('pub(super) fn',1)[0]
        self.assertLess(held.index('write_machine_updates'),held.index('game.give_or_drop'))
        self.assertIn('write_machine_updates(game, &[(key, kind, tank.to_nbt())])',(p/'source-machine-place.rs').read_text())
if __name__=='__main__':unittest.main()
