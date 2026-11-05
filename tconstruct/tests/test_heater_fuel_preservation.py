from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1] / 'mixins/tconstruct'


class HeaterFuelPreservation(unittest.TestCase):
    def test_tick_plans_safe_fuel_before_granting_heat(self):
        source = (ROOT / 'source-machine-runtime.rs').read_text()
        body = source.split('pub(super) fn tick_melter(', 1)[1].split('pub(super) fn casting_at(', 1)[0]
        fuel = body.split('let mut heater_update = None;', 1)[1].split('let mut fluid_update = None;', 1)[0]
        self.assertLess(fuel.index('fuel.filter(super::tconstruct_source_smeltery_fuel::heater_stack_save_safe)'), fuel.index('burn_time('))
        self.assertLess(fuel.index('plan_heater_fuel('), fuel.index('melter.fuel = duration'))
        self.assertIn('heater_before = Some(fuel)', fuel)
        commit = body.index('store::write_many(')
        for guard in ('if let Some(before) = heater_before.as_ref()', '!= heater_state', 'contains_key(&(below.0 >> 4', 'heater_stack_save_safe(now)', 'now.same_item_same_components(before)'):
            self.assertLess(body.index(guard), commit)
        self.assertLess(commit, body.index('live[0] = next'))
        self.assertLess(commit, body.index('if let Some(item) = remainder_drop'))

    def test_helper_preserves_remainder_rules_and_rejects_wire(self):
        source = (ROOT / 'source-smeltery-fuel.rs').read_text()
        self.assertIn('item_from_nbt(&item_to_nbt(item))', source)
        self.assertIn('data.wire.patch().is_some()', source)
        self.assertIn('stack.count < limit.min(64)', source)
        self.assertIn('Some((next, spill))', source)


if __name__ == '__main__':
    unittest.main()
