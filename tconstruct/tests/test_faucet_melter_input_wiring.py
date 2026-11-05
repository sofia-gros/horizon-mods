from pathlib import Path
import unittest

RUNTIME = Path(__file__).resolve().parents[1] / 'mixins/tconstruct/source-machine-runtime.rs'

class FaucetMelterInputWiring(unittest.TestCase):
    def test_dispatch_precedes_legacy_routes(self):
        code = RUNTIME.read_text()
        activate = code[code.index('pub(super) fn activate_faucet('):code.index('pub(super) fn tick_faucet(')]
        tick = code[code.index('pub(super) fn tick_faucet('):code.index('pub(super) fn tick_casting(')]
        self.assertLess(activate.index('melter_lantern_transfer('), activate.index('tank_to_cannon('))
        self.assertLess(tick.index('melter_lantern_transfer('), tick.index('tank_to_cannon('))
        route = code[code.index('fn melter_lantern_transfer('):]
        self.assertIn('patch_melter_tank(&raw_target, &output)', route)
        self.assertNotIn('melter.to_nbt()', route)
        self.assertIn('if !store::write_many(&mut game.world, &changes)', route)
        self.assertIn('if let Some(at) = changed_source { game.container_changed(at); }', route)
        self.assertLess(route.index('process_casting_tank(game, state)'), route.index('let next_target'))
        self.assertLess(route.index('if !store::write_many('), route.index('slots[0] = state.slots[0].clone()'))
        self.assertIn('data.wire.patch().is_some()', route)

    def test_cannon_output_is_guarded_and_preserves_inventory(self):
        route = RUNTIME.read_text().split('fn melter_lantern_transfer(', 1)[1]
        self.assertIn('"tconstruct:seared_fluid_cannon"', route)
        self.assertIn('cannon_at(game, source).is_some_and', route)
        self.assertIn('cannon_input.as_mut()', route)
        self.assertIn('current.item.as_ref().is_none_or', route)
        self.assertIn('tanks::patch_melter_tank(raw, tank)', route)
        self.assertLess(route.index('if !store::write_many('), route.index('if let Some(state) = cannon_input.as_ref()'))

if __name__ == '__main__':
    unittest.main()
