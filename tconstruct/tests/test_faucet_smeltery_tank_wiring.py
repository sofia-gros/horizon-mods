from pathlib import Path
import unittest

RUNTIME = Path(__file__).resolve().parents[1] / 'mixins/tconstruct/source-machine-runtime.rs'

class FaucetSmelteryTankWiring(unittest.TestCase):
    def test_dispatch_precedes_generic_tank_and_drain(self):
        code = RUNTIME.read_text()
        activate = code[code.index('pub(super) fn activate_faucet('):code.index('pub(super) fn tick_faucet(')]
        tick = code[code.index('pub(super) fn tick_faucet('):code.index('pub(super) fn tick_casting(')]
        self.assertLess(activate.index('tank_to_smeltery('), activate.index('tconstruct_source_faucet_tank::activate('))
        self.assertLess(tick.index('tank_to_smeltery('), tick.index('tconstruct_source_faucet_tank::tick('))
        self.assertLess(activate.index('smeltery_to_tank('), activate.index('tconstruct_source_faucet_tank::activate('))
        self.assertLess(tick.index('smeltery_to_tank('), tick.index('tconstruct_source_faucet_tank::tick('))
        old_tick = code[code.index('fn tick_smeltery_faucet('):code.index('/// Start or stop a source faucet.')]
        self.assertIn('raw_controller.get("tank")', old_tick)
        self.assertIn("patch_smeltery_controller_tank(&raw_controller, &smeltery)", old_tick)
        self.assertNotIn('smeltery.to_nbt()', old_tick)

    def test_lantern_drain_input_uses_local_capacity_and_guarded_save(self):
        route = RUNTIME.read_text().split('fn tank_to_smeltery(', 1)[1]
        self.assertIn('"tconstruct:seared_lantern" | "tconstruct:scorched_lantern"', route)
        self.assertIn('Some(("tconstruct:lantern", 50))', route)
        self.assertIn('if tank.to_nbt() != *source_tank', route)
        self.assertIn('patch_smeltery_controller_tank(&raw_controller, &smeltery)', route)
        self.assertIn('capacity != smeltery.tank.capacity', route)

if __name__ == '__main__':
    unittest.main()
