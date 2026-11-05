"""Bounded one-shot fake-metal saved-book handoff, not automatic discovery."""
from pathlib import Path
import tomllib
import unittest

ROOT = Path(__file__).resolve().parents[1]
MIXINS = ROOT / 'mixins/tconstruct'


class FakeMetalBookLoad(unittest.TestCase):
    def test_successful_prepared_world_owns_the_pending_book(self):
        manifest = tomllib.loads((MIXINS / 'horizon-mixin.toml').read_text())
        hook = next(h for h in manifest['injection'] if h['id'] == 'fake-material-book-prepare')
        self.assertEqual(hook['target'], 'horizon_client::game::world_loading::WorldRequest::prepare')
        self.assertEqual(hook['kind'], 'RETURN')
        self.assertEqual(hook['bindings'], [{'name': 'value', 'ty': 'Result<PreparedWorld>', 'value': '$value'}])
        state = next(s for s in manifest['state'] if s['id'] == 'fake-material-book-pending')
        self.assertEqual(state['target'], 'horizon_client_render::chunk_map::ChunkMap')
        self.assertEqual(state['ty'], 'Option<(Vec<String>, Vec<String>)>')
        self.assertEqual(state['initial'], '::core::default::Default::default()')
        source = (MIXINS / hook['helper']).read_text()
        self.assertIn('if let Ok(prepared) = &mut value', source)
        self.assertIn('prepared.world.chunks.', source)
        self.assertIn('&prepared.level.recipe_book', source)
        self.assertNotIn('std::fs', source)

    def test_restore_follows_index_and_waits_outside_guest_scope(self):
        source = (MIXINS / 'source-fake-material-world.rs').read_text()
        self.assertLess(source.index('tconstruct_source_fake_material_book::replace('),
                        source.index('restore_saved_book(game);'))
        restore = source[source.index('pub(super) fn restore_saved_book'):]
        for required in ('game.remote.is_some()', 'game.lan_acting.is_some()',
                         '__horizon_mixins_tconstruct_recipe_snapshot().is_none()',
                         'game.world_tables.as_ref()', 'fake_material_book_pending_mut().take()',
                         '&mut game.level.recipe_book, &tables.stations.recipe_index, pending'):
            self.assertIn(required, restore)
        level = (MIXINS / 'recipe-level-enter.rs').read_text()
        self.assertLess(level.index('game.ensure_datapacks();'), level.index('restore_saved_book(game);'))
        self.assertIn('!game.options.execution_role.is_client()', level)

    def test_restore_does_not_grant_new_recipes_or_read_old_files(self):
        source = (MIXINS / 'source-fake-material-book-load.rs').read_text()
        for required in ('LIMIT: usize = 1024', '.take(LIMIT + 1)', 'known.len() > LIMIT',
                         'candidate(id) && index.contains(id)', 'known.contains(&id)',
                         'book.recipes.contains(&id)', 'hex.len() % 2 == 0'):
            self.assertIn(required, source)
        for forbidden in ('add_recipes(', 'std::fs', 'unsafe', 'static mut'):
            self.assertNotIn(forbidden, source)
        self.assertIn('newly eligible variants are not automatically awarded',
                      (ROOT / 'docs/fake-metal-discovery.md').read_text().lower())


if __name__ == '__main__':
    unittest.main()
