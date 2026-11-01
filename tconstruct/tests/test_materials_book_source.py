"""The initial guide uses pinned source text and the original acquisition recipe."""
import json
import unittest
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
ARCHIVE = ROOT / 'content/assets/tconstruct/materials-and-you.zip'
PREFIX = 'assets/tconstruct/book/materials_and_you/'

class MaterialsBookSourceTest(unittest.TestCase):
    def test_source_static_chapters_and_referenced_pages(self):
        with zipfile.ZipFile(ARCHIVE) as archive:
            index = json.loads(archive.read(PREFIX + 'index.json'))
            sections = [entry['data'] for entry in index if entry.get('data', 'no-load') != 'no-load']
            self.assertIn('sections/introduction.json', sections)
            self.assertIn('sections/tools.json', sections)
            self.assertIn('sections/modifiers.json', sections)
            readable = 0
            for section in sections:
                for entry in json.loads(archive.read(PREFIX + section)):
                    path = PREFIX + 'en_us/' + entry.get('data', '')
                    if path in archive.namelist():
                        page = json.loads(archive.read(path))
                        if page.get('text') or page.get('description'): readable += 1
            self.assertGreater(readable, 15)
            self.assertIn('assets/tconstruct/book/images/covers/materials_and_you_pages.png', archive.namelist())

    def test_book_recipe_uses_book_and_pattern(self):
        recipe = json.loads((ROOT / 'content/data/tconstruct/recipe/materials_and_you.json').read_text())
        self.assertEqual(set(recipe['ingredients']), {'minecraft:book', 'tconstruct:pattern'})
        self.assertEqual(recipe['result']['id'], 'tconstruct:materials_and_you')

if __name__ == '__main__': unittest.main()
