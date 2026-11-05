"""Seared crafting accepts the pinned glass families, not glass panes.

Static recipe/tag/package checks do not simulate host transactions or reload.
"""
import json
from pathlib import Path
import sys
import tempfile
import unittest
from zipfile import ZipFile

from test_survival_reachability import tag_items

ROOT = Path(__file__).resolve().parents[1]
CONTENT = ROOT / 'content'
sys.path.insert(0, str(ROOT / 'tools'))
from package_hmod import package

TAGS = ('glass', 'glass/colorless', 'glass/tinted', 'stained_glass')


class SearedGlassInputs(unittest.TestCase):
    def test_native_glass_tags_merge_both_pinned_sources(self):
        with ZipFile(CONTENT / 'reference/forge-47.4.0-data.zip') as forge:
            for tag in TAGS:
                with self.subTest(tag=tag):
                    vanilla = json.loads(forge.read(f'data/forge/tags/items/{tag}.json'))
                    mod = json.loads((CONTENT / f'reference/tconstruct-tags/data/forge/tags/items/{tag}.json').read_text())
                    actual = json.loads((CONTENT / f'data/forge/tags/item/{tag}.json').read_text())
                    self.assertEqual(actual, {'values': vanilla['values'] + mod['values']})
                    self.assertFalse(actual.get('replace', False))
        self.assertEqual(tag_items('forge:glass/colorless'), {'minecraft:glass', 'tconstruct:clear_glass'})
        all_glass = tag_items('forge:glass')
        self.assertEqual(len(all_glass), 37)
        self.assertFalse(any('pane' in item or 'seared' in item or 'scorched' in item for item in all_glass))
        for item in all_glass:
            if item.startswith('tconstruct:'):
                self.assertTrue((CONTENT / f'data/tconstruct/horizon/block/{item.split(":")[1]}.json').is_file(), item)

    def test_six_recipe_shapes_and_glass_inputs_match_source(self):
        cases = [(f'survival_seared_{name}', f'seared/{name}', 'B', 'forge:glass')
                 for name in ('fuel_gauge', 'fuel_tank', 'ingot_gauge', 'ingot_tank')]
        cases += [('seared_casting_tank', 'seared/seared_casting_tank', 'G', 'forge:glass'),
                  ('survival_seared_seared_glass', 'seared/seared_glass', 'G', 'forge:glass/colorless')]
        with ZipFile(CONTENT / 'reference/tconstruct-recipes.zip') as recipes:
            for native_name, source_name, key, tag in cases:
                with self.subTest(recipe=native_name):
                    source = json.loads(recipes.read(f'data/tconstruct/recipes/smeltery/{source_name}.json'))
                    native = json.loads((CONTENT / f'data/tconstruct/recipe/{native_name}.json').read_text())
                    self.assertEqual(source['key'][key], {'tag': tag})
                    self.assertEqual(native['pattern'], source['pattern'])
                    self.assertEqual(native['type'], source['type'])
                    self.assertEqual(native['result']['id'], source['result']['item'])
                    self.assertEqual(native['result'].get('count', 1), source['result'].get('count', 1))
                    self.assertEqual(tag_items(native['key'][key][1:]), tag_items(tag))
                    self.assertIn('tconstruct:clear_glass', tag_items(native['key'][key][1:]))

    def test_package_contains_complete_native_glass_graph(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'tconstruct.hmod'
            package(path)
            with ZipFile(path) as archive:
                names = [f'data/forge/tags/item/{tag}.json' for tag in TAGS]
                names += ['data/tconstruct/tags/item/source_forge_glass.json',
                          'data/tconstruct/recipe/survival_seared_seared_glass.json']
                for name in names:
                    self.assertEqual(archive.read(name), (CONTENT / name).read_bytes())


if __name__ == '__main__':
    unittest.main()
