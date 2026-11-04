from pathlib import Path
import unittest

P = Path(__file__).resolve().parents[1] / 'mixins/tconstruct/source-machine-runtime.rs'
class Wiring(unittest.TestCase):
    def test_dispatch_and_fallback_guards(self):
        code = P.read_text()
        for section, terminal in [('activate_faucet', 'tick_faucet'), ('tick_faucet', 'tick_casting')]:
            body = code[code.index('pub(super) fn '+section+'('):code.index('pub(super) fn '+terminal+'(')]
            self.assertLess(body.index('tank_to_casting_tank('), body.index('tconstruct_source_faucet_tank::' + ('activate(' if section == 'activate_faucet' else 'tick(')))
        for target in ('casting_tank', 'fluid_cannon'):
            self.assertEqual(code.count('store::read(&game.world,target,"tconstruct:'+target+'").is_some_and(|raw| state.to_nbt() == raw)'), 2)
        self.assertGreaterEqual(code.count('store::read(&game.world,pos,"tconstruct:faucet").is_some_and(|raw| faucet.to_nbt() == raw)'), 2)
if __name__ == '__main__': unittest.main()
