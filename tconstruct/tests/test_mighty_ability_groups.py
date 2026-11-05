"""Source overview/category coverage stays separate from supported recipes."""
import hashlib
import json
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]
BOOK = ROOT / 'content/assets/tconstruct/book/mighty_smelting'
SOURCE_BLOBS = {
    "general": "4fc7043b9012508f314837fffd1cdb78002b2fb9",
    "interact": "af2d31a8b4496b1cc04c978be62245589db5753b",
    "melee": "49a5181cd152cdc1ab960568e3eaee8d3d6d20df",
    "harvest": "0b3e7581936a7c4eabba19ffda789d48b751c147",
    "ranged": "b6bfce0cc979392905f06445aad4195fa848b6a9",
    "armor": "dbe55413ced821f035f35edd699ad5ab4c9068ef"
}


class MightyAbilityGroups(unittest.TestCase):
    def test_six_verbatim_source_pages_follow_the_pinned_section_order(self):
        section = json.loads((BOOK / 'sections/abilities.json').read_text())
        paths = [group['data'] for group in section if 'data' in group]
        self.assertEqual(paths, [f'abilities/{name}.json' for name in SOURCE_BLOBS])
        for name, sha in SOURCE_BLOBS.items():
            with self.subTest(page=name):
                raw = (BOOK / f'en_us/abilities/{name}.json').read_bytes()
                self.assertEqual(hashlib.sha1(b'blob ' + str(len(raw)).encode() + b'\0' + raw).hexdigest(), sha)
                page = json.loads(raw)
                self.assertTrue(page['title'])
                self.assertTrue(page['text'])
        self.assertEqual(len(section), 11)
        self.assertTrue(all('data' not in group for group in section[6:]))

    def test_source_examples_require_explicit_reference_and_empty_category_notices(self):
        labels = dict(line.split('=', 1) for line in (BOOK / 'en_us/language.lang').read_text().splitlines() if '=' in line)
        for name in ('abilities.partial', 'abilities.partial.notice_title', 'abilities.partial.notice',
                     'abilities.partial.reference', 'abilities.partial.empty'):
            self.assertTrue(labels[name].strip(), name)
        self.assertIn('not complete Horizon support', labels['abilities.partial.notice'])
        self.assertIn('Individual recipe pages list eligible tool types', labels['abilities.partial.notice'])
        self.assertIn('may not be implemented', labels['abilities.partial.reference'])
        self.assertIn('guide limitation', labels['abilities.partial.empty'])

    def test_reader_groups_existing_recipe_projection_without_widening_eligibility(self):
        source = (ROOT / 'mixins/tconstruct/source-materials-book.rs').read_text()
        start = source.index('fn modifier_group_pages(')
        body = source[start:source.index('fn incremental_modifier_pages(', start)]
        for text in ('mighty_ability_group(name)', '!= Some(tag)', '!= Some(relative)',
                     'ordinary_modifier_pages(game, &[tag.to_owned()])', 'if recipes.is_empty() { continue; }',
                     'text.push_str(empty)', 'pages.extend(recipes)', 'pages.splice(0..0',
                     '{category}.partial.reference', '{category}.partial.empty', 'split_styled(game,'):
            self.assertIn(text, body)
        self.assertIn('let mighty_abilities = mighty && key == "abilities";', source)
        self.assertIn('if mighty_abilities && !super::tconstruct_materials_book_index::mighty_advice_ready([', source)
        self.assertIn('modifier_group_pages(game, path, root, &section, &labels, puny_upgrades)', source)
        self.assertIn('if upgrades { recipes.extend(incremental_modifier_pages(game, &[tag.to_owned()])); }', body)
        self.assertIn('chapters.push((heading.to_owned(), chapter_pages));', source)
        index = (ROOT / 'mixins/tconstruct/source-materials-book-index.rs').read_text()
        for group in json.loads((BOOK / 'sections/abilities.json').read_text()):
            self.assertIn('"' + group['name'] + '" =>', index)
            self.assertIn('"' + group['extraData']['tconstruct:load_modifiers']['tag'] + '"', index)
