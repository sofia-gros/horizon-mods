"""Crafting Station port guidance uses a complete, bounded source difference."""
import json
from pathlib import Path
import unittest
import zipfile

ROOT = Path(__file__).resolve().parents[1]
BOOK = ROOT / 'content/assets/tconstruct/book/materials_and_you'


class BookDifferenceIngredients(unittest.TestCase):
    def test_port_page_is_reachable_and_uses_unchanged_source_recipe(self):
        pages = json.loads((BOOK / 'sections/introduction.json').read_text())
        names = [page['name'] for page in pages]
        self.assertEqual(names.count('craftingstation_port'), 1)
        self.assertLess(names.index('pattern_craft'), names.index('craftingstation_port'))
        self.assertLess(names.index('craftingstation_port'), names.index('partbuilder_craft'))
        entry = next(page for page in pages if page['name'] == 'craftingstation_port')
        self.assertEqual(entry['type'], 'mantle:crafting')
        page = json.loads((BOOK / 'en_us' / entry['data']).read_text())
        self.assertEqual(page['recipe'], 'tconstruct:tables/crafting_station')
        self.assertEqual(page['grid_size'], 'small')
        self.assertIn('Port Guide', page['title'])
        prose = ' '.join(part['text'] for part in page['description'])
        self.assertIn('does not prove native crafting compatibility', prose)
        self.assertIn('Part Builder or Tinker Station', prose)
        with zipfile.ZipFile(ROOT / 'content/reference/tconstruct-recipes.zip') as archive:
            source = json.loads(archive.read('data/tconstruct/recipes/tables/crafting_station.json'))
        self.assertEqual(source['pattern'], ['p', 'w'])
        self.assertEqual(source['key']['p'], {'item': 'tconstruct:pattern'})
        self.assertEqual(source['key']['w'], {'type': 'forge:difference',
                         'base': {'tag': 'forge:workbenches'},
                         'subtracted': {'item': 'tconstruct:crafting_station'}})
        self.assertEqual(source['result'], {'item': 'tconstruct:crafting_station'})

    def test_production_resolver_preserves_all_exclusions_and_safety_bounds(self):
        helper = (ROOT / 'mixins/tconstruct/source-materials-book-recipe.rs').read_text()
        start = helper.index('fn crafting_ingredient(')
        body = helper[start:helper.index('pub(super) fn resolve(', start)]
        for text in ('Some("forge:difference")', 'object.len() != 3',
                     'object.get("base")?', 'object.get("subtracted")?',
                     'depth + 1', '!subtracted.contains(item)', 'depth > 8',
                     '!choices.is_empty() && choices.len() <= 128'):
            self.assertIn(text, body)
        self.assertIn('crafting_difference_excludes_current_world_members_without_truncating', helper)
        self.assertIn('crafting_difference_requires_bounded_supported_subtraction', helper)
        harness = (ROOT / 'tests/book-wire/src/lib.rs').read_text()
        self.assertIn('difference_crafting_preview_keeps_excluded_items_out_of_book_reply', harness)
