"""Pinned tinted-glass crafting and native Forge/TConstruct input-tag layers."""
import json
from pathlib import Path
import sys
import tempfile
import unittest
from zipfile import ZipFile

ROOT = Path(__file__).resolve().parents[1]
CONTENT = ROOT / 'content'
MATERIALS = ('seared', 'scorched')
TAG = 'data/forge/tags/item/glass/tinted.json'
sys.path.insert(0, str(ROOT / 'tools'))
from package_hmod import package


class TintedGlassCrafting(unittest.TestCase):
    def test_native_recipes_preserve_source_shape_and_tag(self):
        with ZipFile(CONTENT / 'reference/tconstruct-recipes.zip') as source:
            for material in MATERIALS:
                with self.subTest(material=material):
                    name = f'{material}_tinted_glass'
                    original = json.loads(source.read(f'data/tconstruct/recipes/smeltery/{material}/{name}.json'))
                    self.assertEqual(original['type'], 'minecraft:crafting_shaped')
                    self.assertEqual(original['pattern'], [' b ', 'bGb', ' b '])
                    self.assertEqual(original['key'], {
                        'G': {'tag': 'forge:glass/tinted'},
                        'b': {'item': f'tconstruct:{material}_brick'},
                    })
                    self.assertEqual(original['result'], {'item': f'tconstruct:{name}'})
                    expected = dict(original)
                    expected['key'] = {'G': '#forge:glass/tinted', 'b': f'tconstruct:{material}_brick'}
                    expected['result'] = {'id': original['result']['item']}
                    actual = json.loads((CONTENT / f'data/tconstruct/recipe/{name}.json').read_text())
                    self.assertEqual(actual, expected)

    def test_native_tag_combines_both_pinned_layers_without_clear_glass(self):
        with ZipFile(CONTENT / 'reference/forge-47.4.0-data.zip') as forge:
            vanilla = json.loads(forge.read('data/forge/tags/items/glass/tinted.json'))
        modded = json.loads((CONTENT /
            'reference/tconstruct-tags/data/forge/tags/items/glass/tinted.json').read_text())
        self.assertEqual(vanilla, {'values': ['minecraft:tinted_glass']})
        self.assertEqual(modded, {'values': ['tconstruct:clear_tinted_glass']})
        native = json.loads((CONTENT / TAG).read_text())
        self.assertEqual(native, {'values': vanilla['values'] + modded['values']})
        self.assertNotIn('replace', native)
        self.assertNotIn('minecraft:glass', native['values'])
        self.assertNotIn('tconstruct:clear_glass', native['values'])

    def test_package_contains_recipes_tag_and_registered_endpoints(self):
        with tempfile.TemporaryDirectory() as tmp:
            output = Path(tmp) / 'tconstruct.hmod'
            package(output)
            with ZipFile(output) as archive:
                self.assertEqual(archive.read(TAG), (CONTENT / TAG).read_bytes())
                for name in ('clear_tinted_glass', 'seared_tinted_glass', 'scorched_tinted_glass'):
                    with self.subTest(block=name):
                        path = f'data/tconstruct/horizon/block/{name}.json'
                        self.assertEqual(archive.read(path), (CONTENT / path).read_bytes())
                        self.assertEqual(json.loads(archive.read(path))['template'], 'minecraft:tinted_glass')
                for material in MATERIALS:
                    with self.subTest(material=material):
                        path = f'data/tconstruct/recipe/{material}_tinted_glass.json'
                        self.assertEqual(archive.read(path), (CONTENT / path).read_bytes())
                        self.assertIn(f'data/tconstruct/horizon/item/{material}_brick.json', archive.namelist())


if __name__ == '__main__':
    unittest.main()
