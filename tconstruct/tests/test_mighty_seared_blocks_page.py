"""Bounded Seared Blocks guide: source identity, survival route and packaging."""
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


class MightySearedBlocksPage(unittest.TestCase):
    def test_source_slot_and_live_tag_icon(self):
        raw = (ROOT / 'tests/fixtures/mighty_seared_blocks_source.json').read_bytes()
        self.assertEqual(hashlib.sha1(f'blob {len(raw)}\0'.encode() + raw).hexdigest(), '76cede7c3e828c90a5926b960b2571600e352da3')
        source = json.loads(raw)
        page = json.loads((ROOT / 'content' / BOOK / 'en_us/smeltery/searedblocks.json').read_text())
        self.assertEqual(page['title'], source['title'])
        self.assertEqual(source['item'], {'tag': 'tconstruct:seared_blocks'})
        self.assertEqual(page['item'], source['item'])
        entries = json.loads((ROOT / 'content' / BOOK / 'sections/smeltery.json').read_text())
        entry = next(p for p in entries if p['name'] == 'seared_blocks')
        self.assertEqual(entry, {'name': 'seared_blocks', 'type': 'mantle:showcase', 'data': 'smeltery/searedblocks.json'})
        names = [p['name'] for p in entries]
        self.assertEqual(names[names.index('seared_blocks') - 1:names.index('seared_blocks') + 2], ['seared_tanks', 'seared_blocks', 'seared_ladder'])
        text = ' '.join(p['text'] for p in page['text'])
        for phrase in ('icon cycles', 'current world’s Seared Blocks item tag', 'Datapacks may change', 'save/reload and LAN remain unverified'):
            self.assertIn(phrase, text)

    def test_quantities_and_recipe_path(self):
        def recipe(name):
            return json.loads((ROOT / f'content/data/tconstruct/recipe/survival_seared_{name}.json').read_text())
        page = json.loads((ROOT / 'content' / BOOK / 'en_us/smeltery/searedblocks.json').read_text())
        text = ' '.join(p['text'] for p in page['text'])
        for phrase in ('one Clay Ball', 'one Gravel into two Grout', 'one Seared Brick', 'four Seared Bricks in a 2×2 square', 'one Seared Bricks block'):
            self.assertIn(phrase, text)
        grout = recipe('grout')
        self.assertEqual(grout['type'], 'minecraft:crafting_shapeless')
        self.assertEqual(grout['ingredients'], ['minecraft:clay_ball', '#minecraft:sand', 'minecraft:gravel'])
        self.assertEqual(grout['result'], {'id': 'tconstruct:grout', 'count': 2})
        brick = recipe('seared_brick')
        self.assertEqual((brick['type'], brick['ingredient'], brick['result']), ('minecraft:smelting', 'tconstruct:grout', {'id': 'tconstruct:seared_brick'}))
        block = recipe('seared_bricks_from_brick')
        self.assertEqual((block['pattern'], block['key'], block['result']), (['bb', 'bb'], {'b': 'tconstruct:seared_brick'}, {'id': 'tconstruct:seared_bricks'}))
        for name in ('grout', 'seared_bricks'):
            self.assertTrue((ROOT / f'content/data/tconstruct/horizon/block/{name}.json').is_file())

    def test_full_blocks_but_not_decorative_shapes_are_structural(self):
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
            blocks = expand('seared_blocks')
            self.assertIn('tconstruct:seared_bricks', blocks)
            for part in ('floor', 'wall'):
                allowed = expand('smeltery/' + part)
                self.assertTrue(blocks.issubset(allowed))
                for suffix in ('slab', 'stairs', 'wall'):
                    self.assertFalse(any(name.endswith('_' + suffix) for name in allowed))

    def test_native_package_contains_reachable_page_and_recipes(self):
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp) / 'tconstruct.hmod'
            package(out)
            with ZipFile(out) as archive:
                for path in [BOOK / 'en_us/smeltery/searedblocks.json', BOOK / 'sections/smeltery.json', *[Path(f'data/tconstruct/recipe/survival_seared_{name}.json') for name in ('grout', 'seared_brick', 'seared_bricks_from_brick')]]:
                    self.assertEqual(archive.read(str(path)), (ROOT / 'content' / path).read_bytes())
                self.assertIn('smeltery.partial.notice=', archive.read(str(BOOK / 'en_us/language.lang')).decode())


if __name__ == '__main__':
    unittest.main()
