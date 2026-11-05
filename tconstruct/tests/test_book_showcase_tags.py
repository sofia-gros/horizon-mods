"""Structural adapters for the production tag showcase and its existing wire path."""
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]


class BookShowcaseTagWiring(unittest.TestCase):
    def test_world_tags_known_items_and_body_layout_are_wired(self):
        source = (ROOT / 'mixins/tconstruct/source-materials-book.rs').read_text()
        start = source.index('if item.get("tag").is_some()')
        branch = source[start:source.index('let id = super::tconstruct_materials_book_recipe::showcase_item(item)?;', start)]
        self.assertIn('game.world.chunks.__horizon_mixins_tconstruct_resolved_item_tags()', branch)
        self.assertIn('game.world.chunks.__horizon_mixins_tconstruct_melter_known_items()', branch)
        self.assertIn('showcase_tag(item, tags, known)', branch)
        self.assertEqual(source.count('graphic.get("showcase_items").is_some()'), 2)
        self.assertIn('graphic["body_runs"]', source)
        self.assertIn('remaining.unwrap_or_else(|| split_styled', source)
        self.assertIn('id == "tconstruct:creative_slot"', source)

    def test_hover_uses_same_selected_stack_and_keeps_text_for_empty_choices(self):
        source = (ROOT / 'mixins/tconstruct/source-materials-book-draw.rs').read_text()
        start = source.index('if spec.get("showcase_item").is_some()')
        branch = source[start:source.index('if let Some(icon) = spec.get("source_icon")', start)]
        self.assertEqual(branch.count('showcase_display_item(&spec, seconds)'), 1)
        self.assertIn('draw_stack(batch, id, 1, 0, None, x, y)', branch)
        self.assertIn('item_tooltip = Some(stack)', branch)
        self.assertIn('}\n        if let Some(content)', branch)
        self.assertIn('decode_runs', branch)


if __name__ == '__main__':
    unittest.main()
