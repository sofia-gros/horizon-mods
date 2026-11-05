"""Port-adapted tool-tank instructions retain source identity and honest limits."""
import hashlib
import json
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]
BOOK = ROOT / 'content/assets/tconstruct/book/mighty_smelting'


class MightyToolTanks(unittest.TestCase):
    def test_pinned_source_and_existing_section_route(self):
        raw = (ROOT / 'tests/fixtures/mighty_tool_tanks_source.json').read_bytes()
        self.assertEqual(hashlib.sha1(b'blob ' + str(len(raw)).encode() + b'\0' + raw).hexdigest(),
                         '7655880dc9720898dd4f0e2c35c84ec0a2ed81f6')
        page = json.loads((BOOK / 'en_us/tools/tanks.json').read_text())
        self.assertEqual(page['title'], json.loads(raw)['title'])
        self.assertNotIn('image', page)
        section = json.loads((BOOK / 'sections/tools.json').read_text())
        self.assertEqual(next(p['data'] for p in section if p['name'] == 'tanks'), 'tools/tanks.json')
        index = (ROOT / 'mixins/tconstruct/source-materials-book-index.rs').read_text()
        body = index[index.index('pub(super) fn mighty_advice_page'):index.index('pub(super) fn mighty_tool_guide_ready')]
        self.assertIn('"tanks"', body)

    def test_page_covers_existing_transfers_capacity_loss_and_limits(self):
        page = json.loads((BOOK / 'en_us/tools/tanks.json').read_text())
        text = ' '.join(part['text'] for part in page['text'])
        for phrase in ('Partial port guide', '1,000 mB per level', 'output slot empty',
                       'Automatic tries to empty the tool first', 'left-click a fluid layer',
                       'right-click the main tank to empty', 'empty cursor selects',
                       'different fluid metadata', 'Tagged fluid transfers over LAN remain unsupported',
                       'Empty the tool before removing tank capacity', 'discards fluid above the new limit',
                       'final tank handler clears', 'Malformed stored contents reject',
                       'does not enable the original Spitting, Melting or Smashing', 'have not been verified'):
            self.assertIn(phrase, text)

    def test_described_capacity_and_transfer_branches_remain_connected(self):
        rebuild = (ROOT / 'mixins/tconstruct/source-station-world.rs').read_text()
        for text in ('units.checked_mul(1000)', 'pluto_stack_fluid::resize', 'if capacity == 0'):
            self.assertIn(text, rebuild)
        transfers = (ROOT / 'mixins/tconstruct/source-fluid-container.rs').read_text()
        for text in ('if slots[1].is_some() { return false; }', 'if !moved && direction != Direction::EmptyItem',
                     'fluid.id != entry.id || fluid.tag != entry.tag', '(!allow_tags && f.tag.is_some())'):
            self.assertIn(text, transfers)
        pointer = (ROOT / 'mixins/tconstruct/source-smeltery-pointer.rs').read_text()
        self.assertIn('let selected = if right { None } else { index };', pointer)
        self.assertIn('state.tank.move_to_bottom(index)', pointer)
        direct = (ROOT / 'mixins/tconstruct/source-fluid-direct.rs').read_text()
        self.assertIn('if selected.is_some() { Direction::FillItem } else { Direction::EmptyItem }', direct)
