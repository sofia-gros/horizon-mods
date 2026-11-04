from pathlib import Path
import unittest

RUNTIME = Path(__file__).resolve().parents[1] / 'mixins/tconstruct/source-machine-runtime.rs'
class Wiring(unittest.TestCase):
    def test_order_and_safe_fallback(self):
        code = RUNTIME.read_text()
        for fn, end, method in [('activate_faucet','tick_faucet','activate'), ('tick_faucet','tick_casting','tick')]:
            part = code[code.index('pub(super) fn '+fn+'('):code.index('pub(super) fn '+end+'(')]
            self.assertLess(part.index('tank_to_cannon('), part.index('tconstruct_source_faucet_tank::'+method+'('))
        self.assertEqual(code.count('store::read(&game.world,target,"tconstruct:fluid_cannon").is_some_and(|raw| state.to_nbt() == raw)'), 2)
        self.assertGreaterEqual(code.count('store::read(&game.world,pos,"tconstruct:faucet").is_some_and(|raw| faucet.to_nbt() == raw)'), 2)
if __name__ == '__main__': unittest.main()
