"""Slotless recipe categories preserve the intervening illustrated source page."""
import json
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]
BOOK = ROOT / 'content/assets/tconstruct/book/puny_smelting'


class PunySlotlessGroups(unittest.TestCase):
    def test_three_group_identities_match_source_and_damage_tools_stays_separate(self):
        pages = json.loads((BOOK / 'sections/slotless.json').read_text())
        self.assertEqual([page['name'] for page in pages], [
            'group_slotless', 'damage_tools', 'group_bonus_upgrades', 'group_cosmetic'])
        index = (ROOT / 'mixins/tconstruct/source-materials-book-index.rs').read_text()
        start = index.index('fn puny_slotless_group(')
        body = index[start:index.index('fn material_book_tier(', start)]
        for page in [pages[0], pages[2], pages[3]]:
            self.assertIn('"' + page['name'] + '" =>', body)
            self.assertIn('"' + page['data'] + '"', body)
            self.assertIn('"' + page['extraData']['tconstruct:load_modifiers']['tag'] + '"', body)
        self.assertNotIn('"damage_tools" =>', body)
        damage = json.loads((BOOK / 'en_us' / pages[1]['data']).read_text())
        self.assertEqual(damage['image'], {'file': 'tconstruct:book/images/damage_tools.png', 'width': 121, 'height': 65})
        self.assertIn('magma', damage['text'][0]['text'])

    def test_required_notices_qualify_source_advice_and_empty_cosmetic_groups(self):
        labels = dict(line.split('=', 1) for line in (BOOK / 'en_us/language.lang').read_text().splitlines() if '=' in line)
        for suffix in ('', '.notice_title', '.notice', '.reference', '.empty'):
            self.assertTrue(labels['slotless.partial' + suffix].strip())
        self.assertIn('Damage Tools illustration', labels['slotless.partial.notice'])
        self.assertIn('not complete Horizon support', labels['slotless.partial.notice'])
        self.assertIn('may not be implemented', labels['slotless.partial.reference'])
        self.assertIn('guide limitation', labels['slotless.partial.empty'])

    def test_group_recipes_follow_their_overview_and_use_existing_image_overflow(self):
        source = (ROOT / 'mixins/tconstruct/source-materials-book.rs').read_text()
        start = source.index('let slotless_group = if puny_slotless')
        end = source.index('chapters.push((heading.to_owned(), chapter_pages));', start)
        body = source[start:end]
        for text in ('puny_slotless_group)', '!= Some(tag)', '!= Some(relative)',
                     'if tag == "tconstruct:slotless/bonus" { bonus_modifier_pages(game, tag) }',
                     'recipes.extend(incremental_modifier_pages(game, &[tag.to_owned()]))',
                     'value.get("image")', 'graphic["body_runs"]',
                     'remaining = Some(chunks.into_iter().skip(1)',
                     'slotless_group.is_some() && group_recipes.is_empty()',
                     'labels["slotless.partial.reference"]', 'labels["slotless.partial.empty"]',
                     'chapter_pages.extend(group_recipes);', 'chapter_pages.splice(0..0, notices)'):
            self.assertIn(text, body)
        self.assertLess(body.index('for part in parts {'), body.index('chapter_pages.extend(group_recipes);'))
        self.assertIn('let puny_slotless = puny && key == "slotless";', source)
        self.assertIn('if puny_slotless && (!super::tconstruct_materials_book_index::mighty_advice_ready([', source)
        self.assertNotIn('slotless_tags', source)
        self.assertNotIn('chapter_pages.extend(bonus_pages)', source)
