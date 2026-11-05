"""Entity melting advice cannot imply unsupported Horizon acquisition routes."""
import hashlib
import json
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]
BOOK = ROOT / 'content/assets/tconstruct/book/mighty_smelting'
SOURCE_BLOBS = {'sections/entitymelting.json': '6ddf1f3696357036954164f7cb05f62d425ec470', 'en_us/entitymelting/intro.json': 'aed919d71be515687bf92b81648ab1704284b561', 'en_us/entitymelting/entitymelting.json': '3a69f5013f6493968197c9eb8d6629f647ee5345', 'en_us/entitymelting/notable.json': '62ea8b886b1b520f2365da57f248e98ed7c17de4'}


class MightyEntityMeltingGuide(unittest.TestCase):
    def test_three_source_pages_and_section_are_unchanged(self):
        for name, sha in SOURCE_BLOBS.items():
            raw = (BOOK / name).read_bytes()
            self.assertEqual(hashlib.sha1(b'blob ' + str(len(raw)).encode() + b'\0' + raw).hexdigest(), sha)
        section = json.loads((BOOK / 'sections/entitymelting.json').read_text())
        self.assertEqual([(p['name'], p['type'], p['data']) for p in section], [
            ('entitymelting', 'mantle:image_text', 'entitymelting/intro.json'),
            ('entitymelting', 'mantle:text', 'entitymelting/entitymelting.json'),
            ('noteableentities', 'mantle:text', 'entitymelting/notable.json')])
        index = json.loads((BOOK / 'index.json').read_text())
        self.assertEqual(next(p for p in index if p['name'] == 'entity_melting')['data'], 'sections/entitymelting.json')
        notable = json.loads((BOOK / 'en_us/entitymelting/notable.json').read_text())
        self.assertTrue(all(part['linebreak'] for part in notable['text'][:3]))
        self.assertIn('mighty_entity_melting_requires_complete_source_page_identity',
                      (ROOT / 'mixins/tconstruct/source-materials-book-index.rs').read_text())
        self.assertIn('entity_reference_bullets_keep_line_breaks',
                      (ROOT / 'mixins/tconstruct/source-materials-book-layout.rs').read_text())
        self.assertIn('original illustration is not included',
                      (ROOT / 'third_party/mighty-entity-melting-NOTICE.md').read_text())

    def test_mandatory_labels_disclaim_source_acquisition_examples(self):
        labels = dict(line.split('=', 1) for line in (BOOK / 'en_us/language.lang').read_text().splitlines() if '=' in line)
        for suffix in ('', '.notice_title', '.notice', '.reference'):
            self.assertTrue(labels['entity_melting.partial' + suffix].strip())
        notice = labels['entity_melting.partial.notice']
        for text in ('Original-mod reference only', 'does not confirm', 'Silverfish',
                     'villagers/illagers', 'skeletons', 'Ender Dragon',
                     'not supported acquisition promises', 'text-only adaptation'):
            self.assertIn(text, notice)
        self.assertIn('not Horizon instructions', labels['entity_melting.partial.reference'])

    def test_reader_preserves_reference_labels_on_all_text_and_omits_graphics(self):
        source = (ROOT / 'mixins/tconstruct/source-materials-book.rs').read_text()
        for text in ('let mighty_entity = mighty && key == "entity_melting";',
                     'if mighty_entity && (!super::tconstruct_materials_book_index::mighty_advice_ready([',
                     '!labels.get("entity_melting.partial.reference").is_some_and',
                     'mighty_entity_melting_page(', 'format!("[Source] {title}")',
                     'if mighty_entity { graphic = None; }',
                     'text = format!("{}\\n\\n{text}", labels["entity_melting.partial.reference"]);',
                     '("entity_melting.partial.notice_title", "entity_melting.partial.notice")'):
            self.assertIn(text, source)
        self.assertLess(source.index('if mighty_entity { graphic = None; }'), source.index('let mut remaining = None;'))
        self.assertIn('append_parts(&mut text, parts, key == "properties")', source)
        self.assertIn('chapter_pages.splice(0..0, notices)', source)
