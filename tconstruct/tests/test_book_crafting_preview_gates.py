"""Static source-book crafting graphics fail closed without losing page prose."""
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]


class BookCraftingPreviewGates(unittest.TestCase):
    def test_reader_uses_current_world_conditions_and_item_registry(self):
        source = (ROOT / 'mixins/tconstruct/source-materials-book.rs').read_text()
        start = source.index('fn recipe_graphic(')
        body = source[start:source.index('pub(crate) fn source_pages', start)]
        for text in ('__horizon_mixins_tconstruct_recipe_snapshot',
                     '__horizon_mixins_tconstruct_resolved_item_tags',
                     '__horizon_mixins_tconstruct_tank_fluid_context',
                     '__horizon_mixins_tconstruct_source_config_snapshot',
                     'source_conditions_match(values, registries, config)',
                     'rest.items.items.contains_key(item)', 'conditions)'):
            self.assertIn(text, body)
        self.assertIn('.unwrap_or(false)', body)

    def test_renderer_is_optional_and_no_input_is_silently_truncated(self):
        source = (ROOT / 'mixins/tconstruct/source-materials-book.rs').read_text()
        self.assertIn('.and_then(|id| recipe_graphic(game, id,', source)
        helper = (ROOT / 'mixins/tconstruct/source-materials-book-recipe.rs').read_text()
        start = helper.index('pub(super) fn resolve(')
        body = helper[start:helper.index('#[cfg(test)]', start)]
        self.assertNotIn('.take(', body)
        self.assertIn('used.len() != resolved.len()', body)
        self.assertIn('ingredients.len() > grid * grid', body)
        self.assertIn('spec.to_string().len() <= 4096', body)
        self.assertIn('"mantle:crafting_shaped_retextured"', body)
        for case in ('crafting_conditions_gate_before_resolving_icons',
                     'crafting_requires_registered_inputs_outputs_and_live_nonempty_tags',
                     'crafting_rejects_unsupported_serializers_ingredients_and_results',
                     'crafting_never_truncates_oversized_or_malformed_grids',
                     'crafting_bounds_alternatives_depth_and_wire_metadata'):
            self.assertIn(case, helper)
        harness = (ROOT / 'tests/book-wire/src/lib.rs').read_text()
        self.assertIn('validated_crafting_preview_survives_book_reply', harness)
