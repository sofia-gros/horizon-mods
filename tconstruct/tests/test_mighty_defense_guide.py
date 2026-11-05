"""Mighty Defense source guidance stays distinct from gameplay support."""
import hashlib
import json
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]
BOOK = ROOT / 'content/assets/tconstruct/book/mighty_smelting'
SOURCE_BLOBS = {'sections/defense.json': 'cc5474c5be2b48e04a2ea1df5f7b07e245b34482', 'en_us/defense/protection.json': '1373383dfaef7a049b14ee5743f857e486c6ef16', 'en_us/defense/special.json': '571f8965b0dedd082e542facd6ade834a240063f'}


class MightyDefenseGuide(unittest.TestCase):
    def test_pinned_source_section_and_overviews_remain_verbatim(self):
        for name, sha in SOURCE_BLOBS.items():
            raw = (BOOK / name).read_bytes()
            self.assertEqual(hashlib.sha1(b'blob ' + str(len(raw)).encode() + b'\0' + raw).hexdigest(), sha)
        section = json.loads((BOOK / 'sections/defense.json').read_text())
        self.assertEqual([group['name'] for group in section], ['group_protection', 'group_special'])
        index = json.loads((BOOK / 'index.json').read_text())
        self.assertEqual(next(chapter for chapter in index if chapter['name'] == 'defense')['data'], 'sections/defense.json')
        helper = (ROOT / 'mixins/tconstruct/source-materials-book-index.rs').read_text()
        start = helper.index('fn mighty_defense_group(')
        body = helper[start:helper.index('fn puny_upgrade_group(', start)]
        for group in section:
            self.assertIn('"' + group['name'] + '" =>', body)
            self.assertIn('"' + group['extraData']['tconstruct:load_modifiers']['tag'] + '"', body)
            self.assertIn('Some("' + group['data'] + '")', body)
            page = json.loads((BOOK / 'en_us' / group['data']).read_text())
            self.assertTrue(page['title'])
            self.assertTrue(page['text'])
        self.assertIn('_ => return None', body)
        notice = (ROOT / 'third_party/mighty-defense-guide-NOTICE.md').read_text()
        self.assertIn('f4e77d43f39ba1047bf93ae816b7e1beaab514c2', notice)
        self.assertIn('MIT', notice)

    def test_required_notices_distinguish_source_advice_from_usable_recipes(self):
        labels = dict(line.split('=', 1) for line in (BOOK / 'en_us/language.lang').read_text().splitlines() if '=' in line)
        for suffix in ('', '.notice_title', '.notice', '.reference', '.empty'):
            self.assertTrue(labels['defense.partial' + suffix].strip())
        self.assertIn('not completed Horizon behavior', labels['defense.partial.notice'])
        self.assertIn('no supported recipe pages', labels['defense.partial.notice'])
        self.assertIn('not guarantees', labels['defense.partial.reference'])
        self.assertIn('not an application recipe', labels['defense.partial.empty'])

    def test_shared_reader_keeps_category_identity_and_gameplay_gates(self):
        source = (ROOT / 'mixins/tconstruct/source-materials-book.rs').read_text()
        start = source.index('fn modifier_group_pages(')
        body = source[start:source.index('fn modifier_progress_page(', start)]
        for text in ('"defense" => super::tconstruct_materials_book_index::mighty_defense_group(name)',
                     '"abilities" => super::tconstruct_materials_book_index::mighty_ability_group(name)',
                     '"upgrades" => super::tconstruct_materials_book_index::puny_upgrade_group(name)',
                     '{category}.partial.notice', '{category}.partial.reference', '{category}.partial.empty',
                     '!= Some(tag)', '!= Some(relative)', 'ordinary_modifier_pages(game, &[tag.to_owned()])',
                     'let upgrades = category == "upgrades";',
                     'if upgrades { recipes.extend(incremental_modifier_pages(game, &[tag.to_owned()])); }',
                     'text.push_str(empty)', 'pages.extend(recipes)'):
            self.assertIn(text, body)
        self.assertIn('let mighty_defense = mighty && key == "defense";', source)
        self.assertIn('if mighty_defense && !super::tconstruct_materials_book_index::mighty_advice_ready([', source)
        self.assertIn('if mighty_abilities || mighty_defense || puny_upgrades {', source)
        self.assertIn('modifier_group_pages(game, path, root, &section, &labels, key)', source)
        self.assertIn('mighty_defense_keeps_two_exact_source_overviews',
                      (ROOT / 'mixins/tconstruct/source-materials-book-index.rs').read_text())
