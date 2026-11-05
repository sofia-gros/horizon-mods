from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1] / 'mixins/tconstruct'


class CastingTankPreservation(unittest.TestCase):
    def test_save_is_preserving_and_precedes_live_updates(self):
        text = (ROOT / 'source-machine-runtime.rs').read_text()
        body = text.split('pub(super) fn save_casting_tank(', 1)[1].split('fn notify_casting_comparator(', 1)[0]
        self.assertIn('current.to_nbt_preserving(&raw)', body)
        self.assertIn('state.to_nbt_preserving(&raw)', body)
        self.assertNotIn('state.to_nbt()', body)
        for update in ('slots[0] = state.slots[0].clone()', 'notify_casting_comparator(', 'game.container_changed('):
            self.assertLess(body.index('store::write('), body.index(update))

    def test_hopper_rechecks_wire_and_keeps_rollback(self):
        body = (ROOT / 'source-chute-hopper.rs').read_text().split('fn casting_tank_transfer(', 1)[1]
        self.assertIn('data.wire.patch().is_some()', body)
        self.assertIn('slots.get(index).and_then(Option::as_ref).is_none_or(|item| wire_safe(item))', body)
        self.assertIn('save_casting_tank(game, target, &state)', body)
        self.assertIn('*slots = before;', body)
        self.assertLess(body.index('process_casting_tank(game, &mut state)'), body.index('!= target_block'))
        self.assertLess(body.index('!= target_block'), body.index('*slots = after;'))
        self.assertEqual(body.count('state.to_nbt() != before_machine'), 2)


if __name__ == '__main__':
    unittest.main()
