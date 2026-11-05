"""Guard the authoritative multi-alloy tick's fuel and atomic commit boundary."""
import pathlib
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[1]


class AlloyCycleWiring(unittest.TestCase):
    def test_smeltery_only_cycle_uses_staged_randomness_and_tank(self):
        source = (ROOT / 'mixins/tconstruct/source-machine-runtime.rs').read_text()
        tick = source.split('pub(super) fn refresh_smeltery', 1)[1].split('pub(super) fn interact_fuel_tank', 1)[0]
        self.assertIn('if !foundry && state.phase == 2 && state.fuel > 0', tick)
        self.assertIn('let mut random = game.world.random.clone();', tick)
        self.assertIn('let mut candidate = state.tank.clone();', tick)
        self.assertIn('if valid { state.tank = candidate; alloy_random = Some(random); }', tick)
        self.assertEqual(tick.count('state.fuel = state.fuel.saturating_sub(state.fuel_cost);'), 1)
        commit = tick.index('if !store::write_many(&mut game.world, &updates) { return; }')
        self.assertGreater(tick.index('if let Some(random) = alloy_random { game.world.random = random; }'), commit)

    def test_fuel_preflight_uses_same_cycle_without_consuming_randomness(self):
        source = (ROOT / 'mixins/tconstruct/source-melter-world.rs').read_text()
        preflight = source.split('pub(super) fn alloy(', 1)[1].split('pub(super) fn alloy_cycle(', 1)[0]
        self.assertIn('catalog.alloy_cycle(', preflight)
        self.assertIn('&mut |bound| bound.checked_sub(1)', preflight)
        self.assertNotIn('random', preflight)
        cycle = source.split('pub(super) fn alloy_cycle(', 1)[1].split('pub(super) fn fluid_fuel(', 1)[0]
        self.assertIn('conditional_conditions(conditions, registries, config, catalog.known_items)', cycle)


if __name__ == '__main__':
    unittest.main()
