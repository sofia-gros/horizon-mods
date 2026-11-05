"""Travelers projection metadata/index wiring; native checks are deferred."""
import json
from pathlib import Path
import tomllib
import unittest
import zipfile

ROOT = Path(__file__).resolve().parents[1]
MIXINS = ROOT / 'mixins/tconstruct'


class TravelersRecipeBook(unittest.TestCase):
    def test_source_equipment_metadata_is_preserved_by_the_plan(self):
        with zipfile.ZipFile(ROOT / 'content/reference/tconstruct-recipes.zip') as source:
            for name in ('goggles', 'chestplate', 'pants', 'boots', 'shield'):
                recipe = json.loads(source.read(f'data/tconstruct/recipes/tools/armor/travelers/{name}.json'))
                self.assertEqual(recipe['category'], 'equipment')
                self.assertTrue(recipe['show_notification'])
                self.assertNotIn('group', recipe)
        planner = (MIXINS / 'source-travelers-crafting.rs').read_text()
        self.assertIn('pub category: String', planner)
        self.assertIn('pub group: String', planner)
        self.assertIn('pub show_notification: bool', planner)
        self.assertIn('value["show_notification"].as_bool().unwrap_or(true)', planner)

    def test_metadata_and_index_follow_current_world_projection(self):
        source = (MIXINS / 'source-travelers-world.rs').read_text()
        for required in ('game.remote.is_some()', 'generated(game).unwrap_or_default()',
                         'tables.recipes.meta.retain(|id, _| !id.starts_with(PREFIX))',
                         'projection.category, projection.group', 'projection.notify',
                         '&mut tables.stations.recipe_index, &displays',
                         'game.tconstruct_invalidate_travelers_book()'):
            self.assertIn(required, source)
        self.assertLess(source.index('tables.recipes.crafting.push((projection.id, projection.recipe))'),
                        source.index('tconstruct_source_travelers_book::replace('))
        self.assertIn('source_station_world::assemble(', source)

    def test_index_preserves_other_display_numbers_and_reuses_gaps(self):
        helper = (MIXINS / 'source-travelers-book.rs').read_text()
        for required in ('!id.starts_with(PREFIX)', 'displays.len() > 8192',
                         'occupied.sort_unstable()', 'if start > next { break; }',
                         'first_display: next', 'OUTPUTS.contains(&output.as_str())',
                         'station: "minecraft:crafting_table", notify'):
            self.assertIn(required, helper)
        self.assertNotIn('static mut', helper)
        self.assertNotIn('add_recipes(', helper)

    def test_modules_invalidate_book_without_replacing_native_placement(self):
        manifest = tomllib.loads((MIXINS / 'horizon-mixin.toml').read_text())
        modules = {m['id']: m for m in manifest['module']}
        self.assertEqual(modules['source-travelers-book']['file'], 'crates/horizon-client/src/recipe_book.rs')
        self.assertEqual(modules['source-travelers-book']['visibility'], 'pub')
        self.assertEqual(modules['source-travelers-book-ui']['file'], 'crates/horizon-client/src/game/recipe_book.rs')
        ui = (MIXINS / 'source-travelers-book-ui.rs').read_text()
        for required in ('self.forget_book()', 'self.recipe_ui.client_known = None',
                         'self.recipe_ui.ghost.clear()', 'self.recipe_ui.last_clicked = None'):
            self.assertIn(required, ui)
        self.assertNotIn('set_slot(', ui)
        self.assertFalse(any('travelers' in h['id'] and h['target'].endswith('::placement')
                             for h in manifest['injection']))


if __name__ == '__main__':
    unittest.main()
