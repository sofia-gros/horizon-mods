"""Five-input visual guides preserve the existing complete text recipes."""
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]


class ModifierInputDisplay(unittest.TestCase):
    def test_only_supported_bonus_and_ordinary_entries_receive_optional_previews(self):
        source = (ROOT / 'mixins/tconstruct/source-materials-book.rs').read_text()
        for name, following in [('bonus_modifier_pages', 'ordinary_modifier_pages'),
                                ('ordinary_modifier_pages', 'mighty_ability_pages')]:
            body = source[source.index('fn ' + name + '('):source.index('fn ' + following + '(')]
            self.assertIn('modifier_input_page(game, &title, &entry.inputs)', body)
            self.assertIn('if let Some(preview)', body)
            self.assertIn('entry.inputs.iter().enumerate()', body)
            self.assertIn('split_styled(game, &Text::literal(text), 182.0, 15)', body)
            self.assertIn('entry.crystal_alternative', body)
        incremental = source[source.index('fn incremental_modifier_pages('):source.index('fn dynamic_materials(')]
        self.assertNotIn('modifier_input_page', incremental)
        helper = source[source.index('fn modifier_input_page('):source.index('fn bonus_modifier_pages(')]
        for text in ('modifier_input_spec(inputs)?', 'inputs.len() > 3', "require a Tinker's Anvil",
                     'body.len() != 1', 'insertion.len() > 8192', 'This is not a crafting grid',
                     'Repeated ingredients still need separate input slots', 'Details and restrictions follow'):
            self.assertIn(text, helper)

    def test_draw_and_hover_share_one_selected_plain_stack_per_slot(self):
        draw = (ROOT / 'mixins/tconstruct/source-materials-book-draw.rs').read_text()
        start = draw.index('if spec.get("modifier_inputs").is_some()')
        branch = draw[start:draw.index('if let Some(id) = spec.get("image")', start)]
        self.assertEqual(branch.count('modifier_input_items(&spec, seconds)'), 1)
        for text in ('modifier_input_x(index, items.len())', 'let number = (index + 1).to_string()',
                     'let stack = ItemStack::new(*id, 1)', 'game.draw_item(batch, &stack',
                     'item_tooltip = Some(stack)', 'top + 43.0', 'top + 76.0', '.take(11)'):
            self.assertIn(text, branch)
        self.assertNotIn('draw_stack(', branch)
        self.assertNotIn('spec.get("result")', branch)
        self.assertNotIn('tic_materials', branch)

    def test_existing_harness_and_wire_keep_bounded_modifier_metadata(self):
        harness = (ROOT / 'tests/book-wire/src/lib.rs').read_text()
        self.assertIn('source-materials-book-recipe.rs', harness)
        helper = (ROOT / 'mixins/tconstruct/source-materials-book-recipe.rs').read_text()
        for text in ('modifier_inputs_preserve_five_separate_slots_and_cycle_alternatives',
                     'modifier_input_bounds_fail_without_partial_or_misleading_icons',
                     'modifier_input_geometry_fits_one_to_five_numbered_slots'):
            self.assertIn(text, helper)
        wire = (ROOT / 'mixins/tconstruct/source-materials-book-wire.rs').read_text()
        self.assertIn('five_modifier_inputs_and_all_alternatives_survive_guest_transfer', wire)
        self.assertIn('v.len() <= 8192', wire)
