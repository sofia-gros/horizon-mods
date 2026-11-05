"""Progress visuals stay distinct from ordinary one-item-per-slot previews."""
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]


class ModifierProgressDisplay(unittest.TestCase):
    def test_projection_uses_current_supported_entry_and_keeps_complete_text(self):
        source = (ROOT / 'mixins/tconstruct/source-materials-book.rs').read_text()
        helper = source[source.index('fn modifier_progress_page('):source.index('fn incremental_modifier_pages(')]
        for text in ('modifier_progress_spec(inputs, per_item, needed, leftover)?',
                     'modifier_progress_view(&spec, 0)?', 'Return from zero:', 'Return icon identifies the item type',
                     'Each input:', 'One level:', 'From zero:', 'Partial progress changes input and return counts',
                     'body.len() != 1', 'insertion.len() > 8192'):
            self.assertIn(text, helper)
        body = source[source.index('fn incremental_modifier_pages('):source.index('fn dynamic_materials(')]
        self.assertIn('modifier_progress_page(game, &title, &entry.inputs, entry.per_item, entry.needed, entry.leftover.as_deref())', body)
        for text in ('incremental_guide_recipes_with_slotless', 'tool_supported_with_tags',
                     'incremental_limits_supported', 'slotless_incremental_limit',
                     'entry.maximum', 'entry.upgrade_cost', 'entry.crystal_alternative',
                     'split_styled(game, &Text::literal(text), 182.0, 15)'):
            self.assertIn(text, body)
        self.assertNotIn('modifier_input_page', body)

    def test_input_and_conditional_return_share_exact_drawn_tooltip_stacks(self):
        source = (ROOT / 'mixins/tconstruct/source-materials-book-draw.rs').read_text()
        start = source.index('if spec.get("modifier_progress").is_some()')
        body = source[start:source.index('if spec.get("modifier_inputs").is_some()', start)]
        self.assertEqual(body.count('modifier_progress_view(&spec, seconds)'), 1)
        for text in ('view.input', 'if let Some(id) = view.leftover', '"Return type"',
                     'let stack = ItemStack::new(id, 1)', 'game.draw_item(batch, &stack, x, y)',
                     'item_tooltip = Some(stack)', 'top + 76.0', '.take(11)'):
            self.assertIn(text, body)
        self.assertNotIn('draw_stack(', body)
        self.assertNotIn('spec.get("result")', body)

    def test_numeric_and_wire_regressions_are_in_existing_harness(self):
        helper = (ROOT / 'mixins/tconstruct/source-materials-book-recipe.rs').read_text()
        for text in ('incremental_preview_counts_points_items_and_conditional_returns',
                     'incremental_preview_rejects_unknown_values_and_bad_return_items',
                     'from_zero * per_item - needed', 'i32::MAX as u64'):
            self.assertIn(text, helper)
        harness = (ROOT / 'tests/book-wire/src/lib.rs').read_text()
        self.assertIn('incremental_points_and_return_type_survive_book_reply', harness)
        self.assertIn('recipe::modifier_progress_view(&decoded, seconds)', harness)
