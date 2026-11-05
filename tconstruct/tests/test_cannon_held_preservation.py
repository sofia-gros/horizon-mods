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


if __name__ == '__main__':
    unittest.main()
