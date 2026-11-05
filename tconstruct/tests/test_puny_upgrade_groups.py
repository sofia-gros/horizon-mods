"""Puny recipes stay beneath their matching current-world source categories."""
import json
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]
BOOK = ROOT / 'content/assets/tconstruct/book/puny_smelting'


class PunyUpgradeGroups(unittest.TestCase):
    def test_existing_source_order_and_overview_paths_match_bounded_groups(self):
        groups = json.loads((BOOK / 'sections/upgrades.json').read_text())
        self.assertEqual([group['name'] for group in groups], [
            'group_general', 'group_melee', 'group_damage', 'group_harvest', 'group_ranged',
            'group_armor', 'group_armor.helmet', 'group_armor.chestplate',
            'group_armor.leggings', 'group_armor.boots'])
        index = (ROOT / 'mixins/tconstruct/source-materials-book-index.rs').read_text()
        start = index.index('fn puny_upgrade_group(')
        body = index[start:index.index('fn material_book_tier(', start)]
        paths = []
        for group in groups:
            self.assertIn('"' + group['name'] + '" =>', body)
            self.assertIn('"' + group['extraData']['tconstruct:load_modifiers']['tag'] + '"', body)
            if 'data' in group:
                paths.append(group['data'])
                self.assertIn('Some("' + group['data'] + '")', body)
                page = json.loads((BOOK / 'en_us' / group['data']).read_text())
                self.assertTrue(page['title'])
                self.assertTrue(page['text'])
        self.assertEqual(paths, [f'upgrades/{name}.json' for name in ('general', 'melee', 'harvest', 'ranged', 'armor')])

    def test_source_advice_and_empty_groups_have_required_notices(self):
        labels = dict(line.split('=', 1) for line in (BOOK / 'en_us/language.lang').read_text().splitlines() if '=' in line)
        for suffix in ('', '.notice_title', '.notice', '.reference', '.empty'):
            self.assertTrue(labels['upgrades.partial' + suffix].strip())
        self.assertIn('not complete Horizon support', labels['upgrades.partial.notice'])
        self.assertIn('ordinary and incremental', labels['upgrades.partial.notice'])
        self.assertIn('may not be implemented', labels['upgrades.partial.reference'])
        self.assertIn('guide limitation', labels['upgrades.partial.empty'])
        for name in ('damage', 'armor.helmet', 'armor.chestplate', 'armor.leggings', 'armor.boots'):
            self.assertTrue(labels['upgrades.group_' + name])

    def test_both_recipe_forms_follow_their_group_without_flattened_tail(self):
        source = (ROOT / 'mixins/tconstruct/source-materials-book.rs').read_text()
        start = source.index('fn modifier_group_pages(')
        body = source[start:source.index('fn modifier_progress_page(', start)]
        for text in ('if upgrades { "upgrades" } else { "abilities" }',
                     'puny_upgrade_group(name)', 'mighty_ability_group(name)',
                     '!= Some(tag)', '!= Some(relative)',
                     'ordinary_modifier_pages(game, &[tag.to_owned()])',
                     'if upgrades { recipes.extend(incremental_modifier_pages(game, &[tag.to_owned()])); }',
                     'if recipes.is_empty() { continue; }', 'text.push_str(empty)',
                     'pages.extend(titled_pages(&title, &text));', 'pages.extend(recipes);'):
            self.assertIn(text, body)
        self.assertLess(body.index('pages.extend(titled_pages(&title, &text));'), body.index('pages.extend(recipes);'))
        self.assertIn('if mighty_abilities || puny_upgrades {', source)
        self.assertIn('if puny_upgrades && !super::tconstruct_materials_book_index::mighty_advice_ready([', source)
        self.assertNotIn('ordinary_tags', source)
        self.assertIn('chapter_pages.extend(bonus_pages)', source)
        self.assertIn('incremental_modifier_pages(game, &slotless_tags)', source)
