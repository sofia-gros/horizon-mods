from pathlib import Path
import unittest

RUNTIME = Path(__file__).resolve().parents[1] / 'mixins/tconstruct/source-machine-runtime.rs'

class FaucetMelterInputWiring(unittest.TestCase):
    def test_dispatch_precedes_legacy_routes(self):
        code = RUNTIME.read_text()
        activate = code[code.index('pub(super) fn activate_faucet('):code.index('pub(super) fn tick_faucet(')]
        tick = code[code.index('pub(super) fn tick_faucet('):code.index('pub(super) fn tick_casting(')]
        self.assertLess(activate.index('source_to_melter('), activate.index('tank_to_cannon('))
        self.assertLess(tick.index('source_to_melter('), tick.index('tank_to_cannon('))
        route = code[code.index('fn source_to_melter('):]
        self.assertIn('patch_melter_tank(&raw_melter, &melter.tank)', route)
        self.assertNotIn('melter.to_nbt()', route)
        self.assertIn('if !store::write_many(&mut game.world, &changes)', route)
        self.assertIn('if let Some(at) = changed_source { game.container_changed(at); }', route)

if __name__ == '__main__':
    unittest.main()
