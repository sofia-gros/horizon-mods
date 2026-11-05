"""Bounded Pig Iron/Knightmetal routes using existing blocks and storage sprites."""
import json
from pathlib import Path
import sys
import tempfile
import unittest
from zipfile import ZipFile

MOD = Path(__file__).resolve().parents[1]
CONTENT = MOD / 'content'
DATA = CONTENT / 'data/tconstruct'
SOURCE_TAGS = CONTENT / 'reference/tconstruct-tags/data'
METALS = {'pig_iron': 'tconstruct:block/storage/pig_iron/top',
          'knightmetal': 'tconstruct:block/storage/knightmetal'}
ANVILS = ('tinkers_anvil', 'scorched_anvil')
sys.path.insert(0, str(MOD / 'tools'))
from package_hmod import package


class NativeAnvilMetals(unittest.TestCase):
    def test_source_tag_allows_the_registered_blocks_and_existing_textures(self):
        tag = json.loads((SOURCE_TAGS / 'tconstruct/tags/items/anvil_metal.json').read_text())
        for metal, texture in METALS.items():
            self.assertIn('#forge:storage_blocks/' + metal, tag['values'])
            member = json.loads((SOURCE_TAGS / f'forge/tags/items/storage_blocks/{metal}.json').read_text())
            self.assertIn('tconstruct:' + metal + '_block', member['values'])
            self.assertTrue((DATA / f'horizon/block/{metal}_block.json').is_file())
            self.assertTrue((CONTENT / ('assets/tconstruct/textures/' + texture.split(':', 1)[1] + '.png')).is_file())
        pig_model = json.loads((CONTENT / 'assets/tconstruct/models/block/storage/pig_iron.json').read_text())
        self.assertEqual(pig_model['textures']['particle'], METALS['pig_iron'])
        generator = (MOD / 'tools/generate_anvil_variants.py').read_text()
        for metal in METALS:
            self.assertIn("'" + metal + "'", generator)

    def test_four_source_shaped_recipes_preserve_metadata_and_mixed_fallback(self):
        with ZipFile(CONTENT / 'reference/tconstruct-recipes.zip') as source:
            for anvil in ANVILS:
                original = json.loads(source.read(f'data/tconstruct/recipes/tables/{anvil}.json'))
                support = '#tconstruct:anvil_' + ('seared_blocks' if anvil == 'tinkers_anvil' else 'scorched_blocks')
                fallback = json.loads((DATA / f'recipe/zz_{anvil}_mixed_alloys.json').read_text())
                self.assertEqual(fallback['result'], {'id': 'tconstruct:' + anvil})
                for metal, texture in METALS.items():
                    with self.subTest(anvil=anvil, metal=metal):
                        name = anvil + '_' + metal
                        recipe = json.loads((DATA / f'recipe/{name}.json').read_text())
                        self.assertEqual(recipe['type'], 'minecraft:crafting_shaped')
                        self.assertEqual(recipe['pattern'], original['pattern'])
                        self.assertEqual(recipe['key'], {'m': 'tconstruct:' + metal + '_block', 's': support})
                        self.assertEqual(recipe['result']['id'], original['result']['item'])
                        self.assertEqual(recipe['result'].get('count', 1), 1)
                        components = recipe['result']['components']
                        self.assertEqual(components['minecraft:custom_data']['tconstruct:anvil'],
                                         {'Metal': 'tconstruct:' + metal + '_block', 'Variant': metal, 'Texture': texture})
                        self.assertEqual(components['minecraft:item_model'], 'tconstruct:' + name)
                        self.assertIn('tconstruct:' + metal + '_block', fallback['key']['m'])
                        asset = CONTENT / 'assets/tconstruct'
                        self.assertEqual(json.loads((asset / f'items/{name}.json').read_text()),
                                         {'model': {'type': 'minecraft:model', 'model': 'tconstruct:item/' + name}})
                        self.assertEqual(json.loads((asset / f'models/item/{name}.json').read_text()),
                                         {'parent': 'tconstruct:block/table/' + name})
                        self.assertEqual(json.loads((asset / f'models/block/table/{name}.json').read_text()),
                                         {'parent': 'tconstruct:block/table/' + anvil, 'textures': {'particle': texture, 'top': texture}})

    def test_package_contains_four_recipes_and_twelve_variant_models(self):
        with tempfile.TemporaryDirectory() as tmp:
            output = Path(tmp) / 'tconstruct.hmod'
            package(output)
            with ZipFile(output) as archive:
                for anvil in ANVILS:
                    for metal in METALS:
                        name = anvil + '_' + metal
                        for relative in (f'data/tconstruct/recipe/{name}.json',
                                         f'assets/tconstruct/items/{name}.json',
                                         f'assets/tconstruct/models/item/{name}.json',
                                         f'assets/tconstruct/models/block/table/{name}.json'):
                            self.assertEqual(archive.read(relative), (CONTENT / relative).read_bytes())


if __name__ == '__main__':
    unittest.main()
