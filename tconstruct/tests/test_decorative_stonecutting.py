"""Source difference/union recipes, current-world tag discovery and packaging."""
import copy
import hashlib
import json
from pathlib import Path
import sys
import tempfile
import tomllib
import unittest
from zipfile import ZipFile

ROOT = Path(__file__).resolve().parents[1]
DATA = ROOT / 'content/data/tconstruct'
FIXTURES = ROOT / 'tests/fixtures/decorative_stonecutting'
NAMES = {
    'chiseled_scorched_bricks_stonecutting', 'polished_scorched_stone_stonecutting',
    'scorched_bricks_stonecutting', 'seared_bricks_stonecutting',
    'seared_fancy_bricks_stonecutting', 'seared_triangle_bricks_stonecutting',
}
sys.path.insert(0, str(ROOT / 'tools'))
from package_hmod import package


class DecorativeStonecutting(unittest.TestCase):
    def test_exact_source_fixtures_and_recipe_adaptation(self):
        manifest = json.loads((FIXTURES / 'manifest.json').read_text())
        self.assertEqual(set(manifest), {f'{kind}/{name}.json'
            for kind in ('recipes', 'advancements') for name in NAMES})
        for relative, entry in manifest.items():
            self.assertEqual(hashlib.sha256((FIXTURES / relative).read_bytes()).hexdigest(), entry['sha256'])
            self.assertIn('/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/', entry['source'])
        for name in NAMES:
            with self.subTest(name=name):
                source = json.loads((FIXTURES / 'recipes' / (name + '.json')).read_text())
                expected = copy.deepcopy(source)
                expected['ingredient'] = {'type': 'tconstruct:stonecutting_ingredient', 'value': source['ingredient']}
                expected['result'] = {'id': source['result'], 'count': expected.pop('count')}
                native = json.loads((DATA / 'recipe' / (name + '.json')).read_text())
                self.assertEqual(native, expected)
                self.assertEqual(native['type'], 'minecraft:stonecutting')
                self.assertEqual(native['result']['count'], 1)
                self.assertTrue((DATA / 'horizon/block' / (source['result'].split(':')[1] + '.json')).is_file())
                tree = source['ingredient']
                difference = tree[0] if isinstance(tree, list) else tree
                self.assertEqual(difference['type'], 'forge:difference')
                self.assertEqual(difference['subtracted'], {'item': source['result']})
                if name.startswith('seared_'):
                    self.assertEqual(tree[1:], [{'item': 'tconstruct:seared_stone'}])
                    self.assertEqual(difference['base'], {'tag': 'tconstruct:seared_bricks'})
                else:
                    self.assertEqual(difference['base'], {'tag': 'tconstruct:scorched_blocks'})

    def test_discovery_preserves_full_source_tag_and_or_requirements(self):
        for name in NAMES:
            source = json.loads((FIXTURES / 'advancements' / (name + '.json')).read_text())
            expected = copy.deepcopy(source)
            native_id = 'tconstruct:' + name
            expected['criteria']['has_the_recipe']['conditions'] = {'recipes': native_id}
            expected['rewards']['recipes'] = [native_id]
            for criterion in expected['criteria'].values():
                for item in criterion['conditions'].get('items', []):
                    if 'tag' in item:
                        item['items'] = '#' + item.pop('tag')
            actual = json.loads((DATA / 'advancement/recipes/building_blocks' / (name + '.json')).read_text())
            self.assertEqual(actual, expected)
            self.assertEqual(len(actual['requirements']), 1)
            self.assertEqual(set(actual['requirements'][0]), set(actual['criteria']))
            self.assertEqual(actual['parent'], 'minecraft:recipes/root')
            self.assertNotIn('display', actual)

    def test_native_hook_and_world_tag_bridge_are_explicit(self):
        mixins = ROOT / 'mixins/tconstruct'
        manifest = tomllib.loads((mixins / 'horizon-mixin.toml').read_text())
        hooks = [entry for entry in manifest['injection'] if entry['id'] == 'source-stonecutting-ingredient']
        self.assertEqual(len(hooks), 1)
        self.assertEqual(hooks[0]['target'], 'horizon_client::stations::ingredient')
        self.assertEqual(hooks[0]['kind'], 'HEAD')
        self.assertTrue(hooks[0]['cancellable'])
        helper = (mixins / hooks[0]['helper']).read_text()
        self.assertIn('Some("tconstruct:stonecutting_ingredient")', helper)
        self.assertIn('items.item_tag_order.get(tag)', helper)
        self.assertIn('base.retain(|id| !excluded.contains(id))', helper)
        self.assertIn('Break(Ingredient::of(resolved.unwrap_or_default()))', helper)
        for tag in ('tconstruct:seared_bricks', 'tconstruct:scorched_blocks'):
            self.assertIn('"' + tag + '"', (mixins / 'source-advancement-tag.rs').read_text())
            self.assertIn('"' + tag + '"', (ROOT / 'tests/rust/tinker_tool_advancement.rs').read_text())
            self.assertTrue((DATA / 'tags/item' / (tag.split(':')[1] + '.json')).is_file())

    def test_content_package_has_all_six_recipe_and_discovery_pairs(self):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / 'tconstruct.hmod'
            package(output)
            with ZipFile(output) as archive:
                for name in NAMES:
                    for kind in ('recipe', 'advancement/recipes/building_blocks'):
                        path = f'data/tconstruct/{kind}/{name}.json'
                        self.assertEqual(archive.read(path), (ROOT / 'content' / path).read_bytes())


if __name__ == '__main__':
    unittest.main()
