"""The reachable soul-glass guide describes existing, bounded acquisition paths."""
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


class MightySoulGlassPage(unittest.TestCase):
    def test_source_identity_and_reachable_showcase(self):
        raw = (ROOT / 'tests/fixtures/mighty_soul_glass_source.json').read_bytes()
        self.assertEqual(hashlib.sha1(f'blob {len(raw)}\0'.encode() + raw).hexdigest(), '895fc1599e97d700faa7c2ef58b6d04123e30765')
        source = json.loads(raw)
        section = json.loads((ROOT / 'content' / BOOK / 'sections/smeltery.json').read_text())
        entry = next(p for p in section if p['name'] == 'soul_glass')
        self.assertEqual((entry['type'], entry['data']), ('mantle:showcase', 'smeltery/soul_glass.json'))
        page = json.loads((ROOT / 'content' / BOOK / 'en_us' / entry['data']).read_text())
        self.assertEqual((page['title'], page['item']), (source['title'], source['item']))
        names = [p['name'] for p in section]
        self.assertEqual(names[names.index('soul_glass') - 1:names.index('soul_glass') + 2], ['tinted_glass', 'soul_glass', 'seared_drain'])
        for path in ('horizon/block/seared_soul_glass.json', 'recipe/seared_soul_glass.json'):
            self.assertTrue((ROOT / 'content/data/tconstruct' / path).is_file())
        self.assertTrue((ROOT / 'content/assets/tconstruct/items/seared_soul_glass.json').is_file())

    def test_documented_acquisition_matches_pinned_recipes(self):
        with ZipFile(ROOT / 'content/reference/tconstruct-recipes.zip') as archive:
            def recipe(path):
                return json.loads(archive.read('data/tconstruct/recipes/smeltery/' + path + '.json'))
            melting = recipe('melting/soul/sand')
            self.assertEqual(melting['ingredient'], [{'item': 'minecraft:soul_sand'}, {'item': 'minecraft:soul_soil'}])
            self.assertEqual(melting['temperature'], 400)
            self.assertEqual(melting['result'], {'amount': 1000, 'fluid': 'tconstruct:liquid_soul'})
            casting = recipe('casting/soul/glass')
            self.assertEqual(casting['type'], 'tconstruct:casting_basin')
            self.assertNotIn('cast', casting)
            self.assertEqual(casting['fluid'], {'amount': 1000, 'tag': 'tconstruct:liquid_soul'})
            self.assertEqual(casting['result'], 'tconstruct:soul_glass')
        craft = json.loads((ROOT / 'content/data/tconstruct/recipe/seared_soul_glass.json').read_text())
        self.assertEqual(craft['pattern'], [' b ', 'bGb', ' b '])
        self.assertEqual(craft['key'], {'G': 'tconstruct:soul_glass', 'b': 'tconstruct:seared_brick'})
        self.assertEqual(craft['result'], {'id': 'tconstruct:seared_soul_glass'})

    def test_default_wall_membership_and_disclosed_limits(self):
        with ZipFile(ROOT / 'content/reference/tconstruct-block-tags.zip') as archive:
            def expand(tag):
                values = json.loads(archive.read('data/tconstruct/tags/blocks/' + tag + '.json'))['values']
                result = set()
                for value in values:
                    if value.startswith('#tconstruct:'):
                        result.update(expand(value[len('#tconstruct:'):]))
                    elif not value.startswith('#'):
                        result.add(value)
                return result
            self.assertIn('tconstruct:seared_soul_glass', expand('smeltery/wall'))
            self.assertNotIn('tconstruct:seared_soul_glass', expand('smeltery/floor'))
        page = json.loads((ROOT / 'content' / BOOK / 'en_us/smeltery/soul_glass.json').read_text())
        text = ' '.join(p['text'] for p in page['text'])
        for phrase in ('400°C', '1,000 mB', 'empty Casting Basin', 'above, below, left and right', 'not in the floor', 'save/reload and LAN remain unverified', 'does not imply entity melting is available'):
            self.assertIn(phrase, text)

    def test_page_and_partial_notice_are_packaged(self):
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp) / 'tconstruct.hmod'
            package(out)
            with ZipFile(out) as archive:
                path = BOOK / 'en_us/smeltery/soul_glass.json'
                self.assertEqual(archive.read(str(path)), (ROOT / 'content' / path).read_bytes())
                self.assertIn('smeltery.partial.notice=', archive.read(str(BOOK / 'en_us/language.lang')).decode())


if __name__ == '__main__':
    unittest.main()
