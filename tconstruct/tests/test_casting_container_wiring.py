from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1] / 'mixins/tconstruct'

class CastingContainerWiring(unittest.TestCase):
    def test_transfer_precedes_item_swaps_and_commits_after_persistence(self):
        source = (ROOT / 'source-machine-runtime.rs').read_text()
        body = source.split('pub(super) fn interact_casting(', 1)[1].split('\nfn faucet_neighbors(', 1)[0]
        self.assertLess(body.index('if casting.timer > 0'), body.index('transfer_single_with_color'))
        self.assertLess(body.index('transfer_single_with_color'), body.index('if casting.tank.amount != 0 || casting.recipe.is_some() { return true; }'))
        self.assertIn('Direction::EmptyItem', body)
        self.assertIn('casting_recipe_at(game, pos, kind, &next, &offered)', body)
        self.assertIn('Direction::Auto', body)
        self.assertIn('next.recipe = None; next.result = None', body)
        self.assertLess(body.index('store::write_casting(&mut game.world, pos, kind, &next)'), body.index('*game.held_mut() = Some(rest)'))
        self.assertIn('if game.creative', body)
        self.assertLess(body.index('casting_portable_rewrite_safe(stack)'), body.index('transfer_single_with_color'))
        self.assertIn('let held = game.held().cloned();', body)
        self.assertLess(body.index('casting.roundtrips(&raw)'), body.index('store::write_casting(&mut game.world, pos, kind, &next)'))

    def test_both_faucet_sources_reinitialize_the_casting_before_pouring(self):
        source = (ROOT / 'source-machine-runtime.rs').read_text()
        smeltery = source.split('fn tick_smeltery_faucet(', 1)[1].split('pub(super) fn activate_faucet(', 1)[0]
        melter = source.split('pub(super) fn tick_faucet(', 1)[1].split('pub(super) fn tick_casting(', 1)[0]
        for body in (smeltery, melter):
            self.assertLess(body.index('prepare_faucet_casting('), body.rindex('faucet.tick('))
            self.assertLess(body.index('safe_casting_resume('), body.rindex('faucet.tick('))
            self.assertLess(body.index('faucet.to_nbt() == raw'), body.index('prepare_faucet_casting('))
        self.assertIn('casting_faucet_offer(casting, faucet, input)', source)
        self.assertIn('faucet.tick(&mut fluid, None)', smeltery)
        self.assertIn('faucet.tick(input, None)', melter)
        for body in (smeltery, melter):
            self.assertIn('if super::tconstruct_source_melter_state::is_casting_block(name) { return; }', body)
            self.assertLess(body.index('if super::tconstruct_source_melter_state::is_casting_block(name) { return; }'), body.rindex('None)'))

    def test_destroyed_faucet_clears_its_saved_machine_state(self):
        source = (ROOT / 'source-machine-remove.rs').read_text()
        body = source.split('"tconstruct:seared_melter" | "tconstruct:seared_heater"', 1)[1]
        self.assertIn('"tconstruct:seared_faucet"', body)
        self.assertIn('"tconstruct:scorched_faucet"', body)
        self.assertIn('tconstruct_source_melter_store::clear', body)

if __name__ == '__main__':
    unittest.main()
