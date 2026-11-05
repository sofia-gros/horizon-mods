from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1] / 'mixins/tconstruct'


class MeltingSlotPreservation(unittest.TestCase):
    def test_local_edits_preserve_and_rollback_without_overwriting_replacements(self):
        source = (ROOT / 'source-smeltery-click.rs').read_text()
        self.assertNotIn('state.to_nbt()', source)
        self.assertEqual(source.count('state.inventory_to_nbt_preserving(&saved)'), 3)
        self.assertLess(source.index('state.inventory_to_nbt_preserving(&saved)'), source.index('game.menu_ops('))
        self.assertIn('no_retained_wire(&game.inventory)', source)
        rollback = source.split('game.inventory = previous_inventory;', 1)[1]
        for check in ('== block', '== Some(&saved)', 'live == after'):
            self.assertLess(rollback.index(check), rollback.index('*live = previous_live'))
        self.assertIn('game.carried = previous_carried;', rollback)

    def test_guest_quick_move_preserves_before_inventory_publication(self):
        source = (ROOT / 'source-smeltery-guest-guard.rs').read_text()
        self.assertNotIn('state.to_nbt()', source)
        self.assertEqual(source.count('state.inventory_to_nbt_preserving(&tag)'), 2)
        body = source.split('if remaining.as_ref().map_or(0, |item| item.count) != before', 1)[1]
        for check in ('inventory_to_nbt_preserving(&tag)', 'contains_key(', '== block', '== Some(&tag)'):
            self.assertLess(body.index(check), body.index('::write('))
        self.assertLess(body.index('::write('), body.index('hub.guests[gi].inventory = inventory'))


if __name__ == '__main__':
    unittest.main()
