"""Pinned ingredient contracts for the first patterns and expendable casts.

These are data/package checks, not host crafting or datapack reload tests.
"""
import json
from pathlib import Path
import tempfile
import sys
import unittest
from zipfile import ZipFile

from test_survival_reachability import tag_items

ROOT = Path(__file__).resolve().parents[1]
CONTENT = ROOT / 'content'
sys.path.insert(0, str(ROOT / 'tools'))
from package_hmod import package
CASES = (
    ('pattern', 'tables/pattern', 'rods/wooden', 'minecraft:stick', 6),
    ('blank_sand_cast', 'smeltery/sand_cast', 'sand/colorless', 'minecraft:sand', 4),
    ('blank_red_sand_cast', 'smeltery/red_sand_cast', 'sand/red', 'minecraft:red_sand', 4),
)


class PatternCastIngredients(unittest.TestCase):
    def test_native_recipes_preserve_source_ingredients_and_quantities(self):
        with ZipFile(CONTENT / 'reference/tconstruct-recipes.zip') as source:
            for name, source_name, tag, vanilla, count in CASES:
                with self.subTest(recipe=name):
                    original = json.loads(source.read(f'data/tconstruct/recipes/{source_name}.json'))
                    expected = dict(original)
                    expected.pop('show_notification', None)
                    if 'key' in original:
                        expected['key'] = {key: '#' + value['tag'] if 'tag' in value else value['item']
                                           for key, value in original['key'].items()}
                    else:
                        expected['ingredients'] = ['#' + value['tag'] for value in original['ingredients']]
                    expected['result'] = {'id': original['result']['item'], 'count': original['result']['count']}
                    actual = json.loads((CONTENT / f'data/tconstruct/recipe/{name}.json').read_text())
                    self.assertEqual(actual, expected)
                    self.assertEqual(actual['result'], {'id': f'tconstruct:{name}', 'count': count})
                    self.assertIn('#forge:' + tag, list(actual.get('key', {}).values()) + actual.get('ingredients', []))

    def test_default_tags_match_forge_and_do_not_mix_sand_colors(self):
        with ZipFile(CONTENT / 'reference/forge-47.4.0-data.zip') as forge:
            for name, source_name, tag, vanilla, count in CASES:
                with self.subTest(tag=tag):
                    actual = json.loads((CONTENT / f'data/forge/tags/item/{tag}.json').read_text())
                    self.assertEqual(actual, json.loads(forge.read(f'data/forge/tags/items/{tag}.json')))
                    self.assertEqual(actual['values'], [vanilla])
                    self.assertFalse(actual.get('replace', False))
                    self.assertEqual(tag_items('forge:' + tag), {vanilla})
        self.assertTrue(tag_items('forge:sand/colorless').isdisjoint(tag_items('forge:sand/red')))

    def test_native_package_contains_recipes_and_resolvable_default_tags(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'tconstruct.hmod'
            package(path)
            with ZipFile(path) as archive:
                for name, source_name, tag, vanilla, count in CASES:
                    for file in (f'data/tconstruct/recipe/{name}.json', f'data/forge/tags/item/{tag}.json'):
                        self.assertEqual(archive.read(file), (CONTENT / file).read_bytes())


if __name__ == '__main__':
    unittest.main()
