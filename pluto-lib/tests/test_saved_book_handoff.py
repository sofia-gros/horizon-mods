"""Saved-book source boundary checks, not world-opening/runtime proof."""
from pathlib import Path
import tomllib
import unittest

ROOT = Path(__file__).resolve().parents[1]
NATIVE = ROOT / 'mixins/pluto_native_creative'

class SavedBookHandoffTests(unittest.TestCase):
    def test_capture_borrows_prepared_world_at_open_head(self):
        manifest = tomllib.loads((NATIVE / 'horizon-mixin.toml').read_text())
        hook = next(row for row in manifest['injection'] if row['id'] == 'saved-book-before-open')
        self.assertEqual(hook['target'], "horizon_client::game::Game<'_>::finish_open_world")
        self.assertEqual(hook['kind'], 'HEAD')
        self.assertEqual(hook['bindings'], [{'name':'prepared', 'ty':'&crate::game::world_loading::PreparedWorld', 'value':'&prepared'}])
        self.assertNotIn('cancellable', hook)
        capture = next(row for row in manifest['module'] if row['id'] == 'saved-book-capture')
        self.assertEqual(capture['file'], 'crates/horizon-client/src/game.rs')
        self.assertIn('super::world_loading::PreparedWorld', (NATIVE / 'saved-book-capture.rs').read_text())
        source = (NATIVE / 'saved-book-capture.rs').read_text()
        self.assertLess(source.index('handoff::capture('), source.index('.install('))
        self.assertIn('prepared.world.chunks', ''.join(source.split()))
        self.assertNotIn('tconstruct:', source)
    def test_authority_readiness_and_tables_precede_consumption(self):
        source = (NATIVE / 'saved-book-world.rs').read_text()
        take = source.index('.take(&game.world_root, owner)')
        for gate in ('game.remote.is_some()', 'game.options.execution_role.is_client()',
                     'game.lan_acting.is_some()', 'game.lan_acting_uuid.is_some()',
                     'game.lan_acting_entity.is_some()', 'game.options.split_guest',
                     'Readiness::Defer', 'RestoreError::TablesUnavailable'):
            self.assertLess(source.index(gate), take)
        self.assertLess(take, source.index('.plan('))
        self.assertLess(source.index('.plan('), source.index('let book = &mut game.level.recipe_book'))
        self.assertNotIn('add_recipes', source)
        self.assertNotIn('recipe_unlocked', source)
        self.assertIn('ui.invalidate_definitions()', source)
    def test_state_is_only_owned_by_chunkmap_and_clear_discards_it(self):
        manifest = tomllib.loads((NATIVE / 'horizon-mixin.toml').read_text())
        states = [row for row in manifest['state'] if row['id'] == 'saved-book-pending']
        self.assertEqual(len(states), 1)
        self.assertEqual(states[0]['target'], 'horizon_client_render::chunk_map::ChunkMap')
        self.assertEqual(states[0]['constructor'][0]['selector'], 'Self')
        self.assertIn('saved_book_pending_mut().clear()', ''.join((NATIVE / 'saved-book-clear.rs').read_text().split()))
        self.assertNotIn('static', (NATIVE / 'saved-book-state.rs').read_text())

if __name__ == '__main__': unittest.main()
