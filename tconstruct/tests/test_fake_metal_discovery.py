"""Wiring and boundaries of partial fake-metal recipe book support."""
from pathlib import Path
import tomllib
import unittest

ROOT = Path(__file__).resolve().parents[1]
MIXINS = ROOT / 'mixins/tconstruct'


class FakeMetalBook(unittest.TestCase):
    def test_world_local_index_follows_recipe_projection(self):
        source = (MIXINS / 'source-fake-material-world.rs').read_text()
        self.assertLess(source.index('tables.recipes.crafting.extend(generated)'),
                        source.index('tconstruct_source_fake_material_book::replace('))
        self.assertIn('&mut tables.stations.recipe_index, &tables.recipes.crafting', source)
        self.assertIn('game.tconstruct_invalidate_fake_material_book()', source)
        helper = (MIXINS / 'source-fake-material-book.rs').read_text()
        for expected in ('index.recipes.retain', 'known.first_display.checked_add',
                         'display_stacks(id, recipe)', 'book.placements.remove(&entry.id)',
                         'entry.stacks =', 'side * side', 'data.len() == 1'):
            self.assertIn(expected, helper)
        self.assertNotIn('static mut', helper)
        self.assertNotIn('add_recipes(', helper)

    def test_native_modules_and_hooks_are_connected(self):
        manifest = tomllib.loads((MIXINS / 'horizon-mixin.toml').read_text())
        modules = {m['id']: m for m in manifest['module']}
        self.assertEqual(modules['source-fake-material-book']['file'],
                         'crates/horizon-client/src/recipe_book.rs')
        self.assertEqual(modules['source-fake-material-book']['visibility'], 'pub')
        self.assertEqual(modules['source-fake-material-book-ui']['file'],
                         'crates/horizon-client/src/game/recipe_book.rs')
        hooks = {h['id']: h for h in manifest['injection']}
        for name, target in {
            'fake-material-book-build': 'horizon_client_lib::recipe_book::Book::build',
            'fake-material-book-placement': 'horizon_client_lib::crafting::Recipes::placement',
            'fake-material-book-overlay': 'horizon_client_lib::recipe_book_screen::overlay_cells',
        }.items():
            self.assertEqual(hooks[name]['kind'], 'RETURN')
            self.assertEqual(hooks[name]['target'], target)
            self.assertTrue((MIXINS / hooks[name]['helper']).is_file())
        self.assertTrue(hooks['fake-material-book-preview']['cancellable'])
        self.assertEqual(hooks['fake-material-book-preview']['kind'], 'HEAD')

    def test_preview_and_refresh_do_not_move_inventory_or_grant_knowledge(self):
        ui = (MIXINS / 'source-fake-material-book-ui.rs').read_text()
        for expected in ('self.remote.is_some()', 'self.fill_ghost(book_type, &entry)',
                         'self.recipe_ui.client_known = None', 'self.recipe_ui.ghost.clear()',
                         'self.recipe_ui.last_clicked = None'):
            self.assertIn(expected, ui)
        for forbidden in ('menu_session(', 'stack_in_mut(', 'add_recipes(', 'set_slot('):
            self.assertNotIn(forbidden, ui)
        guard = (MIXINS / 'fake-material-book-placement.rs').read_text()
        self.assertIn('tconstruct_source_fake_material_book::marked(recipe)', guard)
        self.assertIn('None', guard)
        self.assertIn('else { value }', guard)


if __name__ == '__main__':
    unittest.main()
