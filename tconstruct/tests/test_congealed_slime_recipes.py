"""Pinned congealed slime crafting, inverse quantities, and packaged inputs.

These are data/packaging checks, not a substitute for host crafting transactions.
"""
import json
from pathlib import Path
import sys
import tempfile
import unittest
from zipfile import ZipFile

ROOT = Path(__file__).resolve().parents[1]
CONTENT = ROOT / 'content'
KINDS = ('earth', 'sky', 'ichor', 'ender')
sys.path.insert(0, str(ROOT / 'tools'))
from package_hmod import package


class CongealedSlimeRecipes(unittest.TestCase):
    def test_recipes_preserve_pinned_shape_tags_and_result(self):
        with ZipFile(CONTENT / 'reference/tconstruct-recipes.zip') as source:
            for kind in KINDS:
                with self.subTest(kind=kind):
                    original = json.loads(source.read(
                        f'data/tconstruct/recipes/common/slime/{kind}/congealed.json'))
                    self.assertEqual(original['pattern'], ['##', '##'])
                    self.assertEqual(original['key'], {'#': {'tag': f'forge:slimeball/{kind}'}})
                    self.assertEqual(original['result'], {'item': f'tconstruct:{kind}_congealed_slime'})
                    expected = dict(original)
                    expected['key'] = {'#': f'#forge:slimeball/{kind}'}
                    expected['result'] = {'id': original['result']['item']}
                    actual = json.loads((CONTENT /
                        f'data/tconstruct/recipe/{kind}_congealed_slime.json').read_text())
                    self.assertEqual(actual, expected)

    def test_native_tags_are_source_exact_and_colors_do_not_mix(self):
        values = []
        for kind in KINDS:
            with self.subTest(kind=kind):
                reference = CONTENT / f'reference/tconstruct-tags/data/forge/tags/items/slimeball/{kind}.json'
                native = CONTENT / f'data/forge/tags/item/slimeball/{kind}.json'
                self.assertEqual(native.read_bytes(), reference.read_bytes())
                tag = json.loads(native.read_text())
                ball = 'minecraft:slime_ball' if kind == 'earth' else f'tconstruct:{kind}_slime_ball'
                self.assertEqual(tag, {'values': [ball]})
                values.extend(tag['values'])
                self.assertTrue((CONTENT / f'data/tconstruct/horizon/block/{kind}_congealed_slime.json').is_file())
        self.assertEqual(len(set(values)), len(KINDS))

    def test_existing_unpacking_conserves_all_four_inputs(self):
        for kind in KINDS:
            with self.subTest(kind=kind):
                recipe = json.loads((CONTENT /
                    f'data/tconstruct/recipe/{kind}_congealed_slime.json').read_text())
                inverse = json.loads((CONTENT /
                    f'data/tconstruct/recipe/{kind}_slimeball_from_congealed.json').read_text())
                self.assertEqual(inverse['ingredients'], [recipe['result']['id']])
                self.assertEqual(inverse['result']['count'], sum(row.count('#') for row in recipe['pattern']))
                tag = json.loads((CONTENT / f'data/forge/tags/item/slimeball/{kind}.json').read_text())
                self.assertIn(inverse['result']['id'], tag['values'])

    def test_native_package_contains_recipes_tags_and_registered_endpoints(self):
        with tempfile.TemporaryDirectory() as tmp:
            target = Path(tmp) / 'tconstruct.hmod'
            package(target)
            with ZipFile(target) as archive:
                for kind in KINDS:
                    with self.subTest(kind=kind):
                        for path in (
                            f'data/tconstruct/recipe/{kind}_congealed_slime.json',
                            f'data/forge/tags/item/slimeball/{kind}.json',
                            f'data/tconstruct/horizon/block/{kind}_congealed_slime.json',
                        ):
                            self.assertEqual(archive.read(path), (CONTENT / path).read_bytes())
                        if kind != 'earth':
                            self.assertIn(f'data/tconstruct/horizon/item/{kind}_slime_ball.json', archive.namelist())


if __name__ == '__main__':
    unittest.main()
