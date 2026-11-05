"""Material conversion regressions, compared with pinned upstream fixtures."""
import json
from pathlib import Path
import unittest
from zipfile import ZipFile

ROOT = Path(__file__).resolve().parents[1]
DATA = ROOT / 'content/data'
REFERENCE_TAGS = ROOT / 'content/reference/tconstruct-tags/data/forge/tags/items'
ALLOYS = (
    'amethyst_bronze', 'cinderslime', 'hepatizon', 'knightmetal', 'knightslime',
    'manyullyn', 'pig_iron', 'queens_slime', 'slimesteel', 'steel',
)
CONVERSIONS = (
    'ingot_from_nuggets', 'nugget_from_ingot', 'block_from_ingots', 'ingot_from_block',
)


def ingredient(value):
    """Independent test expectation, not the production recipe converter."""
    return '#' + value['tag'] if 'tag' in value else value['item']


class MaterialConversions(unittest.TestCase):
    def assert_source_tags(self, tags, *, require_nonreplacement=True):
        for category, material in tags:
            with self.subTest(tag=f'{category}/{material}'):
                expected = json.loads((REFERENCE_TAGS / category / f'{material}.json').read_text())
                actual = json.loads((DATA / 'forge/tags/item' / category / f'{material}.json').read_text())
                self.assertEqual(actual, expected)
                if require_nonreplacement:
                    self.assertFalse(actual.get('replace', False))

    def assert_source_recipes(self, names):
        with ZipFile(ROOT / 'content/reference/tconstruct-recipes.zip') as source:
            for name in names:
                with self.subTest(recipe=name):
                    original = json.loads(source.read(f'data/tconstruct/recipes/common/materials/{name}.json'))
                    expected = dict(original)
                    expected.pop('show_notification', None)
                    if 'key' in expected:
                        expected['key'] = {key: ingredient(value) for key, value in original['key'].items()}
                        self.assertEqual(expected['pattern'], ['###', '#*#', '###'])
                    if 'ingredients' in expected:
                        expected['ingredients'] = [ingredient(value) for value in original['ingredients']]
                    expected['result'] = {
                        'id': original['result']['item'],
                        **({'count': original['result']['count']} if 'count' in original['result'] else {}),
                    }
                    self.assertEqual(json.loads((DATA / f'tconstruct/recipe/{name}.json').read_text()), expected)

    def assert_registered_items(self, names):
        resources = (ROOT / 'tools/resources.py').read_text()
        for name in names:
            with self.subTest(item=name):
                self.assertIn(f"'tconstruct:{name}'", resources)

    def test_alloy_conversions(self):
        for material in ALLOYS:
            with self.subTest(material=material):
                self.assert_source_tags((kind, material) for kind in ('ingots', 'nuggets'))
                self.assert_source_recipes(f'{material}_{kind}' for kind in CONVERSIONS)
                self.assertTrue((DATA / f'tconstruct/horizon/block/{material}_block.json').is_file())
                self.assert_registered_items(f'{material}_{kind}' for kind in ('ingot', 'nugget'))

    def test_netherite_nugget_conversions(self):
        self.assert_source_tags(('nuggets', name) for name in ('netherite', 'netherite_scrap'))
        self.assert_source_recipes((
            'netherite_ingot_from_nuggets', 'netherite_nugget_from_ingot',
            'netherite_scrap_from_nuggets', 'debris_nugget_from_ingot',
        ))
        self.assert_registered_items(('netherite_nugget', 'debris_nugget'))

    def test_rose_gold_recipes(self):
        # Rose Gold keeps its original field-by-field checks and count convention.
        # Do not force its source patterns through the alloy ring-pattern check.
        with ZipFile(ROOT / 'content/reference/tconstruct-recipes.zip') as archive:
            for kind in CONVERSIONS:
                name = 'rose_gold_' + kind
                with self.subTest(recipe=name):
                    source = json.loads(archive.read('data/tconstruct/recipes/common/materials/' + name + '.json'))
                    actual = json.loads((DATA / 'tconstruct/recipe' / (name + '.json')).read_text())
                    self.assertEqual(actual['type'], source['type'])
                    self.assertEqual(actual['category'], source['category'])
                    if 'pattern' in source:
                        self.assertEqual(actual['pattern'], source['pattern'])
                        self.assertEqual(actual['key'], {key: ingredient(value) for key, value in source['key'].items()})
                    else:
                        self.assertEqual(actual['ingredients'], [value['item'] for value in source['ingredients']])
                    self.assertEqual(actual['result'], {
                        'id': source['result']['item'],
                        **({'count': source['result']['count']} if source['result'].get('count', 1) != 1 else {}),
                    })

    def test_rose_gold_tags(self):
        self.assert_source_tags(
            ((kind, 'rose_gold') for kind in ('ingots', 'nuggets')),
            require_nonreplacement=False,
        )


if __name__ == '__main__':
    unittest.main()
