"""Chest acquisition uses current Forge tags without baked-in fallback items."""
import copy
import hashlib
import json
from pathlib import Path
import sys
import tempfile
import unittest
from zipfile import ZipFile

ROOT = Path(__file__).resolve().parents[1]
DATA = ROOT / 'content/data'
FIXTURES = ROOT / 'tests/fixtures/chest_crafting_tags'
CASES = {'part_chest': ('s', 'forge:rods/wooden'), 'tinkers_chest': ('l', 'forge:gems/lapis')}
sys.path.insert(0, str(ROOT / 'tools'))
from package_hmod import package


class ChestCraftingLiveTags(unittest.TestCase):
    def test_exact_pinned_recipe_and_archive_tag_sources(self):
        manifest = json.loads((FIXTURES / 'manifest.json').read_text())
        self.assertEqual(set(manifest), {'recipes/part_chest.json', 'recipes/tinkers_chest.json',
            'tags/chests/wooden.json', 'tags/gems/lapis.json', 'tags/rods/wooden.json'})
        with ZipFile(ROOT / 'content/reference/forge-47.4.0-data.zip') as archive:
            for relative, entry in manifest.items():
                raw = (FIXTURES / relative).read_bytes()
                self.assertEqual(hashlib.sha256(raw).hexdigest(), entry['sha256'])
                if relative.startswith('tags/'):
                    self.assertEqual(entry['archive'], 'content/reference/forge-47.4.0-data.zip')
                    self.assertEqual(archive.read(entry['entry']), raw)
                else:
                    self.assertIn('/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/', entry['source'])

    def test_only_native_syntax_and_existing_symbol_alias_are_adapted(self):
        for name, (symbol, tag) in CASES.items():
            source = json.loads((FIXTURES / 'recipes' / (name + '.json')).read_text())
            expected = copy.deepcopy(source)
            expected['key'] = {('x' if key == symbol else key):
                (value['item'] if 'item' in value else '#' + value['tag']) for key, value in source['key'].items()}
            expected['pattern'] = [row.replace(symbol, 'x') for row in source['pattern']]
            expected['result'] = {'id': source['result']['item']}
            actual = json.loads((DATA / 'tconstruct/recipe' / (name + '.json')).read_text())
            self.assertEqual(actual, expected)
            self.assertEqual(actual['key'], {'p': 'tconstruct:pattern', 'x': '#' + tag,
                'C': '#forge:chests/wooden', 'w': '#minecraft:planks'})
            self.assertEqual(actual['pattern'], [' p ', 'xCx', 'xwx'])
            self.assertEqual(''.join(actual['pattern']).count('x'), 4)
            self.assertEqual(actual['result'], {'id': 'tconstruct:' + name})
            self.assertNotIn('source_forge_', json.dumps(actual))
            self.assertTrue(actual['show_notification'])
            # String tag leaves have no literal fallback union after a pack replaces the tag.
            self.assertIsInstance(actual['key']['x'], str)
            self.assertIsInstance(actual['key']['C'], str)

    def test_default_native_tags_match_source_without_forcing_replacement(self):
        for tag in ('chests/wooden', 'gems/lapis', 'rods/wooden'):
            expected = json.loads((FIXTURES / 'tags' / (tag + '.json')).read_text())
            actual = json.loads((DATA / 'forge/tags/item' / (tag + '.json')).read_text())
            self.assertEqual(actual, expected)
            self.assertNotIn('replace', actual)
        self.assertEqual(json.loads((DATA / 'forge/tags/item/chests/wooden.json').read_text())['values'],
            ['minecraft:chest', 'minecraft:trapped_chest'])

    def test_existing_acquisition_discovery_is_still_pattern_or_recipe(self):
        for name in CASES:
            goal = json.loads((DATA / 'tconstruct/advancement/recipes/decorations' / (name + '.json')).read_text())
            self.assertEqual(goal['criteria']['has_item'], {'trigger': 'minecraft:inventory_changed',
                'conditions': {'items': [{'items': ['tconstruct:pattern']}]}})
            self.assertEqual(goal['criteria']['has_the_recipe'], {'trigger': 'minecraft:recipe_unlocked',
                'conditions': {'recipes': 'tconstruct:' + name}})
            self.assertEqual(goal['requirements'], [['has_item', 'has_the_recipe']])
            self.assertEqual(goal['rewards'], {'recipes': ['tconstruct:' + name]})

    def test_package_contains_real_tags_and_both_recipes(self):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / 'tconstruct.hmod'
            package(output)
            with ZipFile(output) as archive:
                paths = [f'tconstruct/recipe/{name}.json' for name in CASES]
                paths += [f'forge/tags/item/{tag}.json' for tag in ('chests/wooden', 'gems/lapis', 'rods/wooden')]
                for path in paths:
                    self.assertEqual(archive.read('data/' + path), (DATA / path).read_bytes())


if __name__ == '__main__':
    unittest.main()
