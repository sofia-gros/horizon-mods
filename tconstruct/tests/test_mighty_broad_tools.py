"""Source resources and bounded reader wiring for Mighty's broad-tool pages."""
import hashlib
import json
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]
BOOK = ROOT / 'content/assets/tconstruct/book/mighty_smelting'
SOURCE_BLOBS = {
    'broad_axe': '47d81ecf2e5b4bfcb53e100168e415f1658cd293',
    'cleaver': 'c9d570a6bece5beace402ef55614bfa0667e20f8',
    'excavator': 'f39eb438c139f24f67cd99cc4363e468de68bc68',
    'longbow': '83052ac97a7c8e55d6b68ccbb5bbc8b6ce85526b',
    'scythe': '8ce80d74bd5d6a29f4bb7854f2b9a9e5424994b2',
    'sledge_hammer': '7a8f2912ea51693bc5ebf1466b7812f4f65b4772',
    'vein_hammer': 'd1d8810ccd6fb45f7cf2474867de02ff57e63422',
}


class MightyBroadToolPages(unittest.TestCase):
    def test_seven_tool_pages_preserve_pinned_source_content_and_identity(self):
        for name, expected in SOURCE_BLOBS.items():
            with self.subTest(tool=name):
                path = BOOK / 'en_us/tools' / ('tconstruct_' + name + '.json')
                data = path.read_bytes()
                blob = b'blob ' + str(len(data)).encode() + b'\0' + data
                self.assertEqual(hashlib.sha1(blob).hexdigest(), expected)
                value = json.loads(data)
                self.assertEqual(value['tool'], 'tconstruct:' + name)
                self.assertTrue(value['text'])
                self.assertTrue(value['properties'])
        self.assertFalse((BOOK / 'en_us/tools/tconstruct_javelin.json').exists())

    def test_source_group_selectors_and_required_notices_are_retained(self):
        section = json.loads((BOOK / 'sections/tools.json').read_text())
        groups = {page['name']: page for page in section}
        self.assertEqual(groups['group_general']['extraData']['tconstruct:load_tools']['tag'], 'tconstruct:modifiable/broad')
        self.assertEqual(groups['group_ranged']['extraData']['tconstruct:load_tools']['tag'], 'tconstruct:modifiable/ranged/broad')
        labels = dict(line.split('=', 1) for line in (BOOK / 'en_us/language.lang').read_text().splitlines() if '=' in line)
        self.assertEqual(labels['tools.partial'], 'Broad Tools (Partial)')
        for name in ['tools.partial.notice_title', 'tools.partial.notice', 'tools.partial.tool_notice']:
            self.assertTrue(labels[name].strip())
        self.assertIn('Javelins and the tank page remain omitted', labels['tools.partial.notice'])
        self.assertIn('not verified Horizon effect coverage', labels['tools.partial.notice'])
        self.assertIn('component preview is not a guarantee', labels['tools.partial.tool_notice'])

    def test_reader_gates_current_item_parts_source_identity_and_per_tool_notice(self):
        reader = (ROOT / 'mixins/tconstruct/source-materials-book.rs').read_text()
        start = reader.index('let file = format!("{root}en_us/{tool_path}/')
        body = reader[start:reader.index('let fallback =', start)]
        for text in ('if mighty_advice', 'labels.get("tools.partial.tool_notice")',
                     'rest.items.items.contains_key(item)', 'tool_inputs(game, id)',
                     '!inputs.is_empty()', 'inputs.iter().all', 'registered(part)',
                     'mighty_tool_guide_ready(id, registered(id), inputs_available',
                     'value.get("tool")', 'value.get_mut("text")', 'text.insert(0'):
            self.assertIn(text, body)
        self.assertIn('chapter_pages.extend(tool_pages(game, &title, id, Some(&value)))', reader)
        self.assertIn('chapter_pages.insert(group_start, Text::literal(label))', reader)
        request = (ROOT / 'mixins/tconstruct/source-materials-book-request.rs').read_text()
        self.assertIn('source_pages(game, &path, book)', request)
