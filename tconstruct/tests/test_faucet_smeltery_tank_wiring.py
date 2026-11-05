from pathlib import Path
import unittest

RUNTIME = Path(__file__).resolve().parents[1] / 'mixins/tconstruct/source-machine-runtime.rs'

class FaucetSmelteryTankWiring(unittest.TestCase):
    def test_dispatch_precedes_generic_tank_and_drain(self):
        code = RUNTIME.read_text()
        activate = code[code.index('pub(super) fn activate_faucet('):code.index('pub(super) fn tick_faucet(')]
        tick = code[code.index('pub(super) fn tick_faucet('):code.index('pub(super) fn tick_casting(')]
        self.assertLess(activate.index('machine_to_casting('), activate.index('activate_smeltery_faucet('))
        self.assertLess(tick.index('machine_to_casting('), tick.index('tick_smeltery_faucet('))
        self.assertLess(activate.index('smeltery_to_tank('), activate.index('tconstruct_source_faucet_tank::activate('))
        self.assertLess(tick.index('smeltery_to_tank('), tick.index('tconstruct_source_faucet_tank::tick('))
        old_tick = code[code.index('fn tick_smeltery_faucet('):code.index('/// Start or stop a source faucet.')]
        self.assertIn('raw_controller.get("tank")', old_tick)
        self.assertIn("patch_smeltery_controller_tank(&raw_controller, &smeltery)", old_tick)
        self.assertNotIn('smeltery.to_nbt()', old_tick)

    def test_lantern_casting_is_route_local_and_canonical(self):
        route = RUNTIME.read_text().split('fn machine_to_casting(', 1)[1]
        self.assertIn('"tconstruct:seared_lantern" | "tconstruct:scorched_lantern"', route)
        self.assertIn('Tank::from_nbt(&raw, 50)', route)
        self.assertIn('if tank.to_nbt() != raw', route)
        self.assertIn('source_key == "tconstruct:lantern"', route)
        self.assertIn('casting.to_nbt_preserving(&raw_casting)', route)

if __name__ == '__main__':
    unittest.main()
