"""Gadgetry acquisition, bounded reader, advancement parent, and distribution."""
import json
import sys
import tempfile
import unittest
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CONTENT = ROOT / 'content'
BOOK = CONTENT / 'assets/tconstruct/book/tinkers_gadgetry'
sys.path.insert(0, str(ROOT / 'tools'))
from package_hmod import package


class GadgetryAcquisition(unittest.TestCase):
    def test_pinned_recipe_adapts_source_sky_slime_tag(self):
        with zipfile.ZipFile(CONTENT / 'reference/tconstruct-recipes.zip') as archive:
            source = json.loads(archive.read('data/tconstruct/recipes/common/tinkers_gadgetry.json'))
        self.assertEqual(source, {'type': 'minecraft:crafting_shapeless', 'category': 'misc',
            'ingredients': [{'item': 'minecraft:book'}, {'tag': 'forge:slimeball/sky'}],
            'result': {'item': 'tconstruct:tinkers_gadgetry'}})
        recipe = json.loads((CONTENT / 'data/tconstruct/recipe/tinkers_gadgetry.json').read_text())
        self.assertEqual(recipe, {'type': 'minecraft:crafting_shapeless', 'category': 'misc',
            'ingredients': ['minecraft:book', '#forge:slimeball/sky'],
            'result': {'id': 'tconstruct:tinkers_gadgetry', 'count': 1}})
        tag = json.loads((CONTENT / 'data/forge/tags/item/slimeball/sky.json').read_text())
        self.assertIn('tconstruct:sky_slime_ball', tag['values'])
        self.assertEqual(json.loads((CONTENT / 'data/tconstruct/horizon/item/tinkers_gadgetry.json').read_text()),
                         {'template': 'minecraft:book', 'max_stack_size': 1})

    def test_parent_connects_ancient_tools_without_changing_its_criteria(self):
        advancement = json.loads((CONTENT / 'data/tconstruct/advancement/world/tinkers_gadgetry.json').read_text())
        self.assertEqual(advancement['parent'], 'tconstruct:tools/materials_and_you')
        self.assertEqual(advancement['criteria'], {'crafted_book': {
            'trigger': 'minecraft:inventory_changed',
            'conditions': {'items': [{'items': ['tconstruct:tinkers_gadgetry']}]}}})
        self.assertEqual(advancement['requirements'], [['crafted_book']])
        self.assertEqual(advancement['display']['icon'], {'id': 'tconstruct:tinkers_gadgetry'})
        self.assertFalse(advancement['display']['announce_to_chat'])
        self.assertTrue(advancement['display']['show_toast'])
        self.assertEqual(advancement['display']['frame'], 'task')
        self.assertNotIn('rewards', advancement)
        child = json.loads((CONTENT / 'data/tconstruct/advancement/world/ancient_tools.json').read_text())
        self.assertEqual(child['parent'], 'tconstruct:world/tinkers_gadgetry')

    def test_intro_is_bounded_with_notice_on_local_and_server_paths(self):
        self.assertEqual(json.loads((BOOK / 'sections/intro.json').read_text()), [
            {'type': 'mantle:blank'}, {'name': 'intro', 'type': 'mantle:text', 'data': 'intro.json'}])
        self.assertEqual(json.loads((BOOK / 'en_us/intro.json').read_text())['title'], "Melvin's Guide to Gadgets")
        lang = (BOOK / 'en_us/language.lang').read_text()
        self.assertIn('intro.partial.notice_title=Partial edition', lang)
        self.assertIn('includes only Melvin', lang)
        reader = (ROOT / 'mixins/tconstruct/source-materials-book.rs').read_text()
        self.assertIn('if gadgetry && key != "intro" { continue; }', reader)
        self.assertIn('if gadgetry && !super::tconstruct_materials_book_index::mighty_advice_ready', reader)
        self.assertIn('mighty_advice || mighty_smeltery || encyclopedia || gadgetry', reader)
        request = (ROOT / 'mixins/tconstruct/source-materials-book-request.rs').read_text()
        self.assertIn('"tconstruct:tinkers_gadgetry"', request)
        draw = (ROOT / 'mixins/tconstruct/source-materials-book-draw.rs').read_text()
        for suffix in ('cover', 'pages'):
            self.assertIn('gui/tconstruct_book_covers_gadgetry_' + suffix, draw)

    def test_native_package_includes_acquisition_and_reader_assets(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'tconstruct.hmod'
            package(path)
            with zipfile.ZipFile(path) as archive:
                for name in ('data/tconstruct/horizon/item/tinkers_gadgetry.json',
                             'data/tconstruct/recipe/tinkers_gadgetry.json',
                             'data/tconstruct/advancement/world/tinkers_gadgetry.json',
                             'assets/tconstruct/book/tinkers_gadgetry/index.json',
                             'assets/tconstruct/book/tinkers_gadgetry/en_us/intro.json',
                             'assets/tconstruct/book/tinkers_gadgetry/en_us/language.lang',
                             'assets/tconstruct/items/tinkers_gadgetry.json',
                             'assets/tconstruct/models/item/tinkers_gadgetry.json',
                             'assets/tconstruct/textures/item/tinkers_gadgetry.png',
                             'assets/tconstruct/book/images/covers/gadgetry_cover.png',
                             'assets/tconstruct/book/images/covers/gadgetry_pages.png'):
                    self.assertEqual(archive.read(name), (CONTENT / name).read_bytes())
                self.assertIn('data/tconstruct/horizon/item/sky_slime_ball.json', archive.namelist())
                lang = json.loads(archive.read('assets/tconstruct/lang/en_us.json'))
                for key in ('item.tconstruct.tinkers_gadgetry',
                            'advancements.tconstruct.world.tinkers_gadgetry.title',
                            'advancements.tconstruct.world.tinkers_gadgetry.description'):
                    self.assertIn(key, lang)
                tabs = json.loads(archive.read('assets/tconstruct/runtime/creative-tabs.json'))
                self.assertIn('tconstruct:tinkers_gadgetry', json.dumps(tabs))


if __name__ == '__main__':
    unittest.main()
