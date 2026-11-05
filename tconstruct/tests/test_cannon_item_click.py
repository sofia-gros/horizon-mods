from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]


class CannonItemClick(unittest.TestCase):
    def test_ordinary_click_checks_every_native_add_destination(self):
        source = (ROOT / 'mixins/tconstruct/source-machine-runtime.rs').read_text()
        body = source.split('pub(super) fn interact_cannon(', 1)[1].split('/// A formed drain', 1)[0]
        body = body.split('// ItemStack equality', 1)[1]
        self.assertIn('item.count > 0', body)
        self.assertIn('data.wire.patch().is_some()', body)
        self.assertIn('(None, Some(item)) if wire_safe(&item)', body)
        add = body.index('super::inventory_add(')
        for guard in ('game.inventory.len() < 36', 'game.selected >= 36', 'game.inventory.iter().flatten().all(&wire_safe)', 'game.offhand.as_ref().is_none_or(&wire_safe)'):
            self.assertLess(body.index(guard), add)
        no_move = body.index('if item.count == before { return true; }')
        self.assertLess(add, no_move)
        save = body.index('save_cannon(', no_move)
        self.assertLess(no_move, save)
        self.assertLess(save, body.index('game.inventory = inventory'))
        self.assertLess(save, body.index('game.offhand = offhand'))

    def test_fixture_extracts_full_interaction_and_native_planner(self):
        runner = (ROOT / 'tests/run_cannon_held.py').read_text()
        self.assertIn('end = source.index("\\n/// A formed drain", start)', runner)
        self.assertIn('host_game.index("fn inventory_add(")', runner)
        self.assertIn('replace("// __INVENTORY_ADD__", inventory_add)', runner)
        for case in ('cannon_item_click_insert_and_empty_hand_extract_preserve_data', 'cannon_item_click_uses_native_add_order_and_partial_transfer', 'cannon_item_click_no_space_and_unsafe_stacks_do_not_save', 'cannon_item_click_failed_save_preserves_player_inventory'):
            self.assertIn(case, runner)


if __name__ == '__main__':
    unittest.main()
