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


class SupportedBonusGuideWiring(unittest.TestCase):
    def test_puny_bonus_group_reads_current_snapshots_and_existing_pagination(self):
        source = (ROOT / 'mixins/tconstruct/source-materials-book.rs').read_text()
        start = source.index('fn bonus_modifier_pages(')
        body = source[start:source.index('fn dynamic_materials(', start)]
        for accessor in ('modifier_resource_snapshot', 'recipe_snapshot', 'resolved_item_tags'):
            self.assertIn('__horizon_mixins_tconstruct_' + accessor, body)
        for call in ('bonus_guide_registry()', 'supported_guide_tag(raw, tag, &supported)',
                     'bonus_guide_recipes(recipes, &modifiers, &items, &tools',
                     'ingredient_matches(ingredient, item, tags)', 'split_styled(game,',
                     'tconstruct:book-title', 'partial guide'):
            self.assertIn(call, body)
        self.assertIn('puny && key == "slotless"', source)
        self.assertIn('Some("group_bonus_upgrades")', source)
        self.assertIn('/extraData/tconstruct:load_modifiers/tag', source)
        self.assertIn('chapter_pages.extend(bonus_pages)', source)
        self.assertIn('source_pages(game, &path, book)', (ROOT / 'mixins/tconstruct/source-materials-book-request.rs').read_text())

    def test_gameplay_bonus_registry_is_not_duplicated_for_the_book(self):
        rules = (ROOT / 'mixins/tconstruct/source-upgrade-rules.rs').read_text()
        self.assertIn("bonus_guide_registry() -> &'static [(&'static str, i32, i32)] { &BONUS_SLOTS }", rules)
        self.assertIn('pub(super) fn ingredient_matches(', rules)

if __name__ == '__main__':
    unittest.main()
