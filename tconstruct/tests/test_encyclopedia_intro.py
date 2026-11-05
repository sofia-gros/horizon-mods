"""Source acquisition and bounded introductory reader wiring."""
import json
from pathlib import Path
import unittest
import zipfile

ROOT = Path(__file__).resolve().parents[1]
CONTENT = ROOT / 'content'
BOOK = CONTENT / 'assets/tconstruct/book/encyclopedia'

class EncyclopediaIntro(unittest.TestCase):
    def test_pinned_recipe_and_item(self):
        with zipfile.ZipFile(CONTENT / 'reference/tconstruct-recipes.zip') as archive:
            recipe = json.loads(archive.read('data/tconstruct/recipes/common/encyclopedia.json'))
        self.assertEqual(recipe, {'type':'tconstruct:casting_table', 'cast':{'item':'minecraft:book'},
            'cast_consumed':True, 'cooling_time':57, 'fluid':{'amount':90,'tag':'forge:molten_gold'},
            'result':'tconstruct:encyclopedia'})
        self.assertEqual(json.loads((CONTENT / 'data/tconstruct/horizon/item/encyclopedia.json').read_text()),
            {'template':'minecraft:book','max_stack_size':1})
        for p in ('assets/tconstruct/items/encyclopedia.json','assets/tconstruct/models/item/encyclopedia.json',
                  'assets/tconstruct/textures/item/encyclopedia.png', 'assets/tconstruct/book/images/covers/encyclopedia_cover.png',
                  'assets/tconstruct/book/images/covers/encyclopedia_pages.png'):
            self.assertTrue((CONTENT / p).is_file(), p)

    def test_intro_and_notice_in_both_reader_paths(self):
        index = json.loads((BOOK / 'index.json').read_text())
        self.assertIn('intro', [chapter['name'] for chapter in index])
        section = json.loads((BOOK / 'sections/intro.json').read_text())
        self.assertEqual([p['data'] for p in section], ['intro/foreward.json','intro/focus.json'])
        for name in ('foreward','focus'):
            self.assertTrue(json.loads((BOOK / f'en_us/intro/{name}.json').read_text())['text'])
        lang = (BOOK / 'en_us/language.lang').read_text()
        self.assertIn('intro.partial.notice_title=Partial edition', lang)
        self.assertIn('Slimesuit materials, tools, armor equipment, modifiers and fluid effects are not yet included', lang)
        reader = (ROOT / 'mixins/tconstruct/source-materials-book.rs').read_text()
        self.assertIn('if encyclopedia && key != "intro" && encyclopedia_material.is_none() { continue; }', reader)
        self.assertIn('mighty_entity || encyclopedia || gadgetry', reader)
        self.assertIn('labels.get("intro.partial.notice").copied()', reader)
        request = (ROOT / 'mixins/tconstruct/source-materials-book-request.rs').read_text()
        self.assertIn('"tconstruct:encyclopedia"', request)
        self.assertIn('"tconstruct:encyclopedia"', reader)

if __name__ == '__main__': unittest.main()
