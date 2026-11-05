from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1] / 'mixins/tconstruct'


class CannonHeldPreservation(unittest.TestCase):
    def test_held_fluid_path_patches_without_rewriting_inventory(self):
        text = (ROOT / 'source-machine-runtime.rs').read_text()
        body = text.split('pub(super) fn interact_cannon(', 1)[1].split('\n    let held = game.held().cloned();', 1)[0]
        self.assertNotIn('save_cannon(', body)
        self.assertIn('state.tank_to_nbt_preserving(&raw_cannon)', body)
        self.assertIn('fresh.tank_to_nbt_preserving(&raw_cannon)', body)
        self.assertIn('held.same_item_same_components(&held_before)', body)
        self.assertLess(body.index('let Some(raw_cannon)'), body.index('game.ensure_datapacks()'))
        for mutation in ('notify_casting_comparator(', 'game.container_changed(', '*game.held_mut()', 'game.give_or_drop('):
            self.assertLess(body.index('store::write('), body.index(mutation))
        self.assertNotIn('block_entities.get_mut', body)

class CannonItemPreservation(unittest.TestCase):
    def test_item_save_persists_before_live_slot_and_notifications(self):
        text = (ROOT / 'source-machine-runtime.rs').read_text()
        body = text.split('pub(super) fn save_cannon(', 1)[1].split('pub(super) fn interact_cannon(', 1)[0]
        self.assertIn('current.tank_to_nbt_preserving(&raw)', body)
        self.assertIn('state.to_nbt_preserving(&raw)', body)
        self.assertNotIn('state.to_nbt()', body)
        for mutation in ('slots[0] = state.item.clone()', 'notify_casting_comparator(', 'game.container_changed('):
            self.assertLess(body.index('store::write('), body.index(mutation))

    def test_hopper_checks_retained_wire_and_keeps_rollback(self):
        text = (ROOT / 'source-chute-hopper.rs').read_text()
        body = text.split('fn cannon_transfer(', 1)[1].split('// The source sided inventory', 1)[0]
        self.assertIn('data.wire.patch().is_some()', body)
        self.assertIn('slots.get(index).and_then(Option::as_ref).is_none_or(|item| wire_safe(item))', body)
        self.assertIn('save_cannon(game,target,&cannon)', body)
        self.assertIn('*slots = before;', body)
        callback = body.index('game.native_inventory_move_item(')
        stage = body.index('let mut after = before.clone()')
        for check in ('!= hopper_block', '!= target_block', '!= Some(&saved)', 'contains_key(&(hopper_pos.0 >> 4', 'contains_key(&(target.0 >> 4'):
            self.assertLess(callback, body.index(check, callback))
            self.assertLess(body.index(check, callback), stage)
        self.assertLess(body.index('hopper_pos == target'), callback)
        self.assertLess(body.index('cannon.tank_to_nbt_preserving(&saved)'), callback)


if __name__ == '__main__':
    unittest.main()
