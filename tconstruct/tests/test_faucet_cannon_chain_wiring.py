from pathlib import Path
import unittest

RUNTIME = Path(__file__).resolve().parents[1] / 'mixins/tconstruct/source-machine-runtime.rs'

class CannonChainWiring(unittest.TestCase):
    def test_chain_precedes_legacy_routes_and_commits_before_notifications(self):
        code = RUNTIME.read_text()
        activate = code[code.index('pub(super) fn activate_faucet('):code.index('pub(super) fn tick_faucet(')]
        tick = code[code.index('pub(super) fn tick_faucet('):code.index('pub(super) fn tick_casting(')]
        for body in [activate, tick]:
            self.assertLess(body.index('cannon_to_cannon('), body.index('tank_to_cannon('))
        route = code[code.index('fn cannon_to_cannon('):]
        self.assertIn('patch_melter_tank(&raw_input, &input.tank)', route)
        self.assertIn('patch_melter_tank(&raw_output, &output.tank)', route)
        self.assertIn('current.to_nbt() == *raw && item_safe(&current)', route)
        self.assertLess(route.index('store::write_many('), route.index('notify_casting_comparator('))
        self.assertNotIn('get_mut(', route)

if __name__ == '__main__': unittest.main()
