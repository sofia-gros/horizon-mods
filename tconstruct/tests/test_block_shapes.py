"""Block-shape recipes and registrations against pinned upstream fixtures."""
import json
from pathlib import Path
import unittest
from zipfile import ZipFile

ROOT = Path(__file__).resolve().parents[1]
DATA = ROOT / 'content/data/tconstruct'
SHAPES = {
    'slab': (['BBB'], 6),
    'stairs': (['B  ', 'BB ', 'BBB'], 4),
    'wall': (['BBB', 'BBB'], 6),
    'fence': (['BbB', 'BbB'], 6),
}
MASONRY = (
    ('seared', 'seared_bricks', ('slab', 'stairs', 'wall')),
    ('seared', 'seared_cobble', ('slab', 'stairs', 'wall')),
    ('seared', 'seared_stone', ('slab', 'stairs')),
    ('seared', 'seared_paver', ('slab', 'stairs')),
    ('scorched', 'scorched_bricks', ('slab', 'stairs', 'fence')),
    ('scorched', 'scorched_road', ('slab', 'stairs')),
)


class BlockShapes(unittest.TestCase):
    def assert_source_recipe(self, source, reference, name):
        # Keep expected adaptation independent of the production converter.
        original = json.loads(source.read(f'data/tconstruct/recipes/{reference}.json'))
        expected = dict(original)
        expected.pop('show_notification')
        expected['key'] = {key: value['item'] for key, value in original['key'].items()}
        expected['result'] = {'id': original['result']['item'], 'count': original['result']['count']}
        actual = json.loads((DATA / f'recipe/{name}.json').read_text())
        self.assertEqual(actual, expected)
        return actual

    def test_masonry_shapes(self):
        with ZipFile(ROOT / 'content/reference/tconstruct-recipes.zip') as source:
            for family, material, shapes in MASONRY:
                with self.subTest(material=material):
                    self.assertTrue((DATA / f'horizon/block/{material}.json').is_file())
                    for shape in shapes:
                        with self.subTest(shape=shape):
                            name = f'{material}_{shape}'
                            actual = self.assert_source_recipe(source, f'smeltery/{family}/{name}', name)
                            pattern, count = SHAPES[shape]
                            self.assertEqual(actual['pattern'], pattern)
                            key = {'B': f'tconstruct:{material}'}
                            if material == 'scorched_bricks' and shape == 'fence':
                                key['b'] = 'tconstruct:scorched_brick'
                            self.assertEqual(actual['key'], key)
                            self.assertEqual(actual['result'], {'id': f'tconstruct:{name}', 'count': count})
                            self.assertTrue((DATA / f'horizon/block/{name}.json').is_file())
        self.assertIn("'tconstruct:scorched_brick'", (ROOT / 'tools/resources.py').read_text())

    def test_firewood_shapes(self):
        with ZipFile(ROOT / 'content/reference/tconstruct-recipes.zip') as source:
            for wood in ('blazewood', 'nahuatl'):
                for shape in ('fence', 'slab', 'stairs'):
                    with self.subTest(wood=wood, shape=shape):
                        name = f'{wood}_{shape}'
                        self.assert_source_recipe(source, f'common/firewood/{name}', name)
                        self.assertTrue((DATA / f'horizon/block/{wood}.json').is_file())
                        self.assertTrue((DATA / f'horizon/block/{name}.json').is_file())

    def test_slimewood_shapes(self):
        with ZipFile(ROOT / 'content/reference/tconstruct-recipes.zip') as source:
            for wood in ('greenheart', 'skyroot', 'bloodshroom', 'enderbark'):
                for shape in ('slab', 'stairs'):
                    with self.subTest(wood=wood, shape=shape):
                        actual = self.assert_source_recipe(
                            source, f'world/wood/{wood}/{shape}', f'{wood}_planks_{shape}',
                        )
                        self.assertEqual(actual['result']['count'], 6 if shape == 'slab' else 4)
                        for item in (*actual['key'].values(), actual['result']['id']):
                            self.assertTrue((DATA / f'horizon/block/{item.split(":", 1)[1]}.json').is_file(), item)


if __name__ == '__main__':
    unittest.main()
