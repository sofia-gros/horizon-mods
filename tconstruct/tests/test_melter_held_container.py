"""Wiring contract for the staged direct-container Melter interaction."""
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]

class MelterHeldContainer(unittest.TestCase):
    def test_direct_transfer_precedes_menu_and_inventory_follows_save(self):
        source = (ROOT / 'mixins/tconstruct/source-machine-use.rs').read_text()
        branch = source.split('"tconstruct:seared_melter" => {', 1)[1].split('"tconstruct:copper_gauge" =>', 1)[0]
        self.assertLess(branch.index('plan_melter_held'), branch.index('game.open_menu'))
        self.assertIn('if !store::write', branch)
        self.assertIn('plan.saved) { return Break(Ok(false)); }', branch)
        for mutation in ('game.give_or_drop', '*game.held_mut()', 'game.container_changed'):
            self.assertLess(branch.index('if !store::write'), branch.index(mutation))
        self.assertIn('if game.creative', branch)
        self.assertIn('if plan.drained', branch)
        self.assertIn('game.lan.is_none() && game.lan_acting_uuid.is_none()', branch)
        self.assertLess(source.index('if game.remote.is_some()'), source.index('plan_melter_held'))
        self.assertLess(source.index('KeyCode::ShiftLeft'), source.index('plan_melter_held'))

    def test_preparation_cannot_switch_transfer_or_menu_targets(self):
        source = (ROOT / 'mixins/tconstruct/source-machine-use.rs').read_text()
        branch = source.split('"tconstruct:seared_melter" => {', 1)[1].split('"tconstruct:copper_gauge" =>', 1)[0]
        prepare = branch.index('game.ensure_datapacks()')
        self.assertLess(branch.index('let raw_before'), prepare)
        self.assertLess(branch.index('let held_before'), prepare)
        self.assertLess(prepare, branch.index('if !unchanged(game)'))
        self.assertLess(branch.index('if !unchanged(game)'), branch.index('plan_melter_held'))
        self.assertEqual(branch.count('if !unchanged(game)'), 2)
        self.assertLess(branch.rindex('if !unchanged(game)'), branch.index('game.open_menu'))
        for check in ('game.remote.is_none()', 'contains_key(&(pos.0 >> 4', '== state', '== raw_before', '== creative', '== local', 'horizon_client::pluto_stack_storage::same_item_snapshot(before, now)'):
            self.assertIn(check, branch)

    def test_plan_rewrites_only_tank_and_stays_non_mutating(self):
        source = (ROOT / 'mixins/tconstruct/source-fluid-container.rs').read_text()
        plan = source.split('pub(super) fn plan_melter_held', 1)[1].split('/// Apply the same bucket rules', 1)[0]
        self.assertIn('raw: &horizon_world::nbt::Tag', plan)
        self.assertIn('held: &ItemStack', plan)
        self.assertIn('Melter::from_nbt(raw)?', plan)
        self.assertIn('data.wire.patch().is_some()', plan)
        self.assertIn('!casting_portable_rewrite_safe(held)', plan)
        self.assertIn('plan_single_tank_container(raw.get("tank")?', plan)
        self.assertIn('saved.insert("tank".into(), tank)', plan)
        self.assertNotIn('store::write', plan)

if __name__ == '__main__':
    unittest.main()
