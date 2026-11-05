from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1] / 'mixins/tconstruct'


class FuelWithdrawalPreservation(unittest.TestCase):
    def test_both_machine_ticks_validate_debit_before_granting_fuel(self):
        source = (ROOT / 'source-machine-runtime.rs').read_text()
        smeltery = source.split('pub(super) fn refresh_smeltery(', 1)[1].split('pub(super) fn interact_fuel_tank(', 1)[0]
        melter = source.split('let mut fluid_update = None;', 1)[1].split('let mut lit_update', 1)[0]
        for body, prefix in ((smeltery, 'state'), (melter, 'melter')):
            plan = body.index('Tank::drain_fuel_preserving(')
            self.assertLess(plan, body.index(f'{prefix}.fuel = fuel.duration'))
            self.assertNotIn('tank.take(fuel.fluid.amount)', body)
            self.assertNotIn('tank.to_nbt_preserving(&raw_tank)', body)
        self.assertIn('else { continue; }', smeltery)
        self.assertNotIn('return;', melter)
        self.assertIn('if let Some(next_tank) = Tank::drain_fuel_preserving', melter)
        self.assertIn('if let Some(update) = fluid_update { updates.push(update); }', source)
        self.assertIn('if let Some(update) = fuel_update { updates.push(update); }', source)

    def test_fuel_codec_refuses_aliases_instead_of_removing_them(self):
        source = (ROOT / 'source-melter-state.rs').read_text()
        body = source.split('pub(super) fn to_nbt_preserving_fuel(', 1)[1].split('pub(super) fn from_nbt(', 1)[0]
        for alias in ('FluidName', 'Amount', 'Tag', 'Tank', 'Fluid'):
            self.assertIn(f'"{alias}"', body)
        self.assertNotIn('fields.remove', body)
        self.assertLess(body.index('tank.to_nbt_preserving_fuel(raw).as_ref() != Some(raw)'), body.index('tank.take(amount)?'))


if __name__ == '__main__':
    unittest.main()
