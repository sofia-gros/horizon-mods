"""Material casting buckets and fluid descriptions must cycle together."""
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]


class MaterialFluidTooltips(unittest.TestCase):
    def test_reader_preserves_world_recipe_condition_and_item_gates(self):
        source = (ROOT / 'mixins/tconstruct/source-materials-book.rs').read_text()
        start = source.index('let mut fluid_options = Vec::new()')
        body = source[start:source.index('let supported_types:', start)]
        for text in ('hide_in_book', 'MaterialFluidRecipe::parse(bytes, &conditions)',
                     'if recipe.material != id', 'fluid_ids(&recipe.fluid)', 'filled_bucket(&fluid)',
                     'rest.items.items.contains_key(bucket.as_str())',
                     'fluid_options.push((bucket, fluid_name(&fluid)))',
                     'material_fluid_options(&fluid_options, &title, &description)', 'sidebar.push(entry)'):
            self.assertIn(text, body)
        self.assertNotIn('first_fluid', body)
        self.assertNotIn('"choices":buckets', body)

    def test_draw_selects_once_before_reading_icon_and_hover(self):
        source = (ROOT / 'mixins/tconstruct/source-materials-book-draw.rs').read_text()
        start = source.index('if let Some(sidebar) = panel.get("sidebar")')
        body = source[start:source.index('let Some(groups) = panel.get("groups")', start)]
        self.assertEqual(body.count('material_sidebar_entry(entry, seconds)'), 1)
        self.assertLess(body.index('material_sidebar_entry'), body.index('entry.get("item")'))
        self.assertLess(body.index('material_sidebar_entry'), body.index('entry.get("tooltip")'))
        for text in ('game.draw_item(batch, &stack, sidebar_x, y)', 'entry.get("tooltip")',
                     'entry.get("materials")', 'entry.get("display_tool")'):
            self.assertIn(text, body)

    def test_regressions_cover_cycle_bounds_and_legacy_entries(self):
        helper = (ROOT / 'mixins/tconstruct/source-materials-book-recipe.rs').read_text()
        for name in ('material_fluid_options_keep_bucket_and_description_together',
                     'material_fluid_options_reject_invalid_and_over_budget_pairs',
                     'material_sidebar_legacy_entries_remain_unchanged'):
            self.assertIn(name, helper)
        harness = (ROOT / 'tests/book-wire/src/lib.rs').read_text()
        self.assertIn('material_fluid_sidebar_pairs_survive_book_reply', harness)
