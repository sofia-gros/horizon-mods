"""Bounded colored slime registration, crafting and packaged casting inputs.

These data checks do not cover slime physics or in-game transactions.
"""
import hashlib
import json
from pathlib import Path
import sys
import tempfile
import unittest
from zipfile import ZipFile

MOD = Path(__file__).resolve().parents[1]
CONTENT = MOD / 'content'
KINDS = ('sky', 'ichor', 'ender')
sys.path.insert(0, str(MOD / 'tools'))
from package_hmod import package


class ColoredSlimeBlocks(unittest.TestCase):
    def test_source_crafting_and_unpacking_preserve_colors_and_quantity(self):
        with ZipFile(CONTENT / 'reference/tconstruct-recipes.zip') as source:
            for kind in KINDS:
                with self.subTest(kind=kind):
                    source_root = f'data/tconstruct/recipes/common/slime/{kind}/'
                    shaped = json.loads(source.read(source_root + 'slimeblock.json'))
                    self.assertEqual(shaped['pattern'], ['###'] * 3)
                    self.assertEqual(shaped['key'], {'#': {'tag': f'forge:slimeball/{kind}'}})
                    self.assertEqual(shaped['result'], {'item': f'tconstruct:{kind}_slime'})
                    shaped['key'] = {'#': f'#forge:slimeball/{kind}'}
                    shaped['result'] = {'id': f'tconstruct:{kind}_slime'}
                    native = CONTENT / f'data/tconstruct/recipe/{kind}_slime.json'
                    self.assertEqual(json.loads(native.read_text()), shaped)
                    reverse = json.loads(source.read(source_root + 'slimeball_from_block.json'))
                    self.assertEqual(reverse['result'], {'item': f'tconstruct:{kind}_slime_ball', 'count': 9})
                    reverse['ingredients'] = [f'tconstruct:{kind}_slime']
                    reverse['result'] = {'id': f'tconstruct:{kind}_slime_ball', 'count': 9}
                    native = CONTENT / f'data/tconstruct/recipe/{kind}_slimeball_from_block.json'
                    self.assertEqual(json.loads(native.read_text()), reverse)
                    tag = json.loads((CONTENT / f'data/forge/tags/item/slimeball/{kind}.json').read_text())
                    self.assertEqual(tag['values'], [f'tconstruct:{kind}_slime_ball'])

    def test_registered_blocks_have_source_properties_and_self_drops(self):
        for kind, color in zip(KINDS, (31, 15, 24)):
            with self.subTest(kind=kind):
                block = json.loads((CONTENT / f'data/tconstruct/horizon/block/{kind}_slime.json').read_text())
                self.assertEqual(block, {'template': 'minecraft:slime_block', 'properties': {
                    'hardness': 0.0, 'explosion_resistance': 0.0, 'requires_tool': False,
                    'sounds': 'minecraft:slime_block', 'friction': 0.8,
                    'map_color': color, 'light': 10 if kind == 'ichor' else 0}})
                loot = json.loads((CONTENT / f'data/tconstruct/loot_table/blocks/{kind}_slime.json').read_text())
                self.assertEqual(loot['pools'], [{'condition': {'type': 'minecraft:survives_explosion'},
                    'entries': [{'type': 'minecraft:item', 'name': f'tconstruct:{kind}_slime'}], 'rolls': 1}])
                item = json.loads((CONTENT / f'assets/tconstruct/items/{kind}_slime.json').read_text())
                self.assertEqual(item['model']['model'], f'tconstruct:item/{kind}_slime')

    def test_source_models_and_sprites_match_manifest(self):
        rows = (MOD / 'docs/upstream-colored-slime.sha256').read_text().splitlines()
        self.assertEqual(len(rows), 12)
        for row in rows:
            digest, path = row.split('  ', 1)
            self.assertEqual(hashlib.sha256((CONTENT / path).read_bytes()).hexdigest(), digest, path)
        for kind in KINDS:
            model = json.loads((CONTENT / f'assets/tconstruct/models/block/slime/sticky/{kind}.json').read_text())
            self.assertEqual(model['render_type'], 'minecraft:translucent')
            self.assertEqual(model['textures']['texture'], f'tconstruct:block/slime/storage/{kind}')
        ichor = json.loads((CONTENT / 'assets/tconstruct/models/block/slime/sticky/ichor.json').read_text())
        self.assertEqual(ichor['textures']['inner'], 'minecraft:block/lava_still')
        self.assertEqual(ichor['elements'][1]['from'], [3, 3, 3])

    def test_native_package_contains_registered_casting_outputs_and_recipes(self):
        with tempfile.TemporaryDirectory() as tmp:
            target = Path(tmp) / 'tconstruct.hmod'
            package(target)
            with ZipFile(target) as archive:
                for kind in KINDS:
                    for path in (
                        f'data/tconstruct/horizon/block/{kind}_slime.json',
                        f'data/tconstruct/loot_table/blocks/{kind}_slime.json',
                        f'data/tconstruct/recipe/{kind}_slime.json',
                        f'data/tconstruct/recipe/{kind}_slimeball_from_block.json',
                        f'assets/tconstruct/items/{kind}_slime.json',
                        f'assets/tconstruct/models/block/slime/sticky/{kind}.json',
                        f'assets/tconstruct/textures/block/slime/storage/{kind}.png',
                    ):
                        self.assertEqual(archive.read(path), (CONTENT / path).read_bytes())
                    casting = json.loads(archive.read(
                        f'assets/tconstruct/source-packs/tconstruct-original-pack/data/tconstruct/recipes/smeltery/casting/slime/{kind}/block.json'))
                    self.assertEqual(casting['result'], f'tconstruct:{kind}_slime')
                    self.assertEqual(casting['cast'], {'item': f'tconstruct:{kind}_congealed_slime'})
                    self.assertTrue(casting['cast_consumed'])
                    self.assertEqual(casting['fluid']['amount'], 1250)


if __name__ == '__main__':
    unittest.main()
