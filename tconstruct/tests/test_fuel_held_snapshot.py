from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]


class FuelHeldSnapshot(unittest.TestCase):
    def test_world_use_preserves_and_rechecks_before_commit(self):
        text = (ROOT / 'mixins/tconstruct/source-machine-runtime.rs').read_text()
        body = text.split('pub(super) fn interact_fuel_tank(', 1)[1].split('pub(super) fn casting_tank_at(', 1)[0]
        prepare = body.index('game.ensure_datapacks()')
        self.assertLess(body.index('let Some(raw_tank)'), prepare)
        self.assertLess(body.index('let held_before'), prepare)
        self.assertIn('plan_single_tank_container(', body)
        self.assertNotIn('to_nbt_preserving_fuel(', body)
        self.assertNotIn('tank.to_nbt()', body)
        commit = body.index('store::write(')
        for guard in ('!= block_state', '!= Some(&raw_tank)', 'game.creative != creative', 'contains_key(&(pos.0 >> 4', 'current.same_item_same_components(&held_before)', 'current.wire_patch().is_none()', 'data.wire.patch().is_some()', 'game.lan_acting_uuid.is_none()) != local'):
            self.assertLess(prepare, body.index(guard, prepare))
            self.assertLess(body.index(guard, prepare), commit)
        for mutation in ('game.give_or_drop(', '*game.held_mut()'):
            self.assertLess(commit, body.index(mutation))

    def test_fixture_executes_production_route(self):
        runner = (ROOT / 'tests/run_drain_container.py').read_text()
        self.assertIn('source.index("pub(super) fn interact_fuel_tank(")', runner)
        self.assertIn('replace("// __PRODUCTION_HELD_FUEL__", held_fuel)', runner)
        for case in ('held_fuel_all_eight_tanks_preserve_data_and_inventory', 'held_fuel_aliases_wire_and_failed_save_never_publish', 'held_fuel_callback_changes_preserve_newer_state'):
            self.assertIn(case, runner)


if __name__ == '__main__':
    unittest.main()
