"""Adapted Mighty fluid-access advice stays reachable, honest, and packaged."""
import hashlib
import json
from pathlib import Path
import sys
import tempfile
import unittest
from zipfile import ZipFile

ROOT = Path(__file__).resolve().parents[1]
BOOK = Path('assets/tconstruct/book/mighty_smelting')
sys.path.insert(0, str(ROOT / 'tools'))
from package_hmod import package


class MightyFluidAccessPages(unittest.TestCase):
    def test_pinned_source_identity_and_source_order(self):
        entries = json.loads((ROOT / 'content' / BOOK / 'sections/smeltery.json').read_text())
        expected = {
            'seared_drain': '150136b370657ce84eb7efd6accd786323010722',
            'seared_duct': '9f0be35f7839f430541748fd4b15a72d2d5418c4',
        }
        self.assertEqual([p['name'] for p in entries if p['name'] in expected], list(expected))
        for name, sha in expected.items():
            raw = (ROOT / f'tests/fixtures/mighty_{name}_source.json').read_bytes()
            self.assertEqual(hashlib.sha1(f'blob {len(raw)}\0'.encode() + raw).hexdigest(), sha)
            entry = next(p for p in entries if p['name'] == name)
            self.assertEqual((entry['type'], entry['data']), ('mantle:showcase', f'smeltery/{name}.json'))
            page = json.loads((ROOT / 'content' / BOOK / 'en_us' / entry['data']).read_text())
            source = json.loads(raw)
            self.assertEqual((page['title'], page['item']), (source['title'], source['item']))
            self.assertTrue(all(p['text'] for p in page['text']))
            self.assertTrue((ROOT / f'content/data/tconstruct/horizon/block/{name}.json').is_file())
            self.assertTrue((ROOT / f'content/assets/tconstruct/items/{name}.json').is_file())

    def test_limitations_and_player_instructions_are_visible(self):
        def text(name):
            page = json.loads((ROOT / 'content' / BOOK / f'en_us/smeltery/{name}.json').read_text())
            return ' '.join(p['text'] for p in page['text'])
        drain = text('seared_drain')
        for phrase in ('formed smeltery wall', 'Casting Table or Basin', 'activate the faucet',
                       'click a fluid', 'Copper Cans', 'external-pipe integration', 'unverified'):
            self.assertIn(phrase, drain)
        duct = text('seared_duct')
        for phrase in ('filter and its fluid display are not implemented', 'does not filter fluids',
                       'bottom fluid selected in the controller', 'not a working filter guide', 'unverified'):
            self.assertIn(phrase, duct)

    def test_reader_keeps_partial_notice_and_packages_pages(self):
        reader = (ROOT / 'mixins/tconstruct/source-materials-book.rs').read_text()
        self.assertIn('mighty_smeltery_page', reader)
        self.assertIn('smeltery.partial.notice', reader)
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp) / 'tconstruct.hmod'
            package(out)
            with ZipFile(out) as archive:
                for name in ('seared_drain', 'seared_duct'):
                    path = BOOK / f'en_us/smeltery/{name}.json'
                    self.assertEqual(archive.read(str(path)), (ROOT / 'content' / path).read_bytes())


if __name__ == '__main__':
    unittest.main()
