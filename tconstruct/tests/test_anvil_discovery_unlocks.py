"""Source Anvil discovery mapped to the existing bounded native recipe variants."""
import copy
import hashlib
import json
from pathlib import Path
import sys
import tempfile
import unittest
from zipfile import ZipFile

MOD = Path(__file__).resolve().parents[1]
DATA = MOD / 'content/data/tconstruct'
FIXTURES = MOD / 'tests/fixtures/anvil_discovery_unlocks'
METALS = ('amethyst_bronze', 'cinderslime', 'hepatizon', 'knightmetal', 'knightslime',
          'manyullyn', 'netherite', 'pig_iron', 'queens_slime', 'rose_gold', 'slimesteel')
COMPAT = ('bronze', 'constantan', 'electrum', 'invar', 'nicrosil', 'pewter')
ANVILS = ('tinkers_anvil', 'scorched_anvil')
sys.path.insert(0, str(MOD / 'tools'))
from package_hmod import package


def recipes(anvil):
    return ([(metal, anvil + '_' + metal) for metal in METALS] +
            [('compat_' + metal, anvil + '_compat_' + metal) for metal in COMPAT] +
            [('mixed_alloys', 'zz_' + anvil + '_mixed_alloys')])


def metal_block(metal):
    return 'minecraft:netherite_block' if metal == 'netherite' else 'tconstruct:' + metal + '_block'


class AnvilDiscoveryUnlocks(unittest.TestCase):
    def test_source_tag_and_all_variant_recipe_events_remain_or_alternatives(self):
        hashes = json.loads((FIXTURES / 'sha256.json').read_text())
        self.assertEqual(set(hashes), set(ANVILS))
        for anvil in ANVILS:
            with self.subTest(anvil=anvil):
                raw = (FIXTURES / (anvil + '.json')).read_bytes()
                self.assertEqual(hashlib.sha256(raw).hexdigest(), hashes[anvil])
                original = json.loads(raw)
                self.assertEqual(original['criteria']['has_item']['conditions'],
                                 {'items': [{'tag': 'tconstruct:anvil_metal'}]})
                self.assertEqual(original['requirements'], [['has_item', 'has_the_recipe']])
                expected = copy.deepcopy(original)
                expected['criteria']['has_item']['conditions'] = {'items': [{'items': '#tconstruct:anvil_metal'}]}
                del expected['criteria']['has_the_recipe']
                for variant, recipe in recipes(anvil):
                    expected['criteria']['has_recipe_' + variant] = {
                        'trigger': 'minecraft:recipe_unlocked', 'conditions': {'recipes': 'tconstruct:' + recipe}}
                expected['requirements'] = [list(expected['criteria'])]
                expected['rewards']['recipes'] = ['tconstruct:' + recipe for _, recipe in recipes(anvil)]
                native = json.loads((DATA / f'advancement/recipes/decorations/{anvil}.json').read_text())
                self.assertEqual(native, expected)
                self.assertEqual(len(native['rewards']['recipes']), 18)
                self.assertNotIn('display', native)
                existing = {path.stem for path in (DATA / 'recipe').glob(anvil + '_*.json')}
                existing.update(path.stem for path in (DATA / 'recipe').glob('zz_' + anvil + '_*.json'))
                self.assertEqual(existing, {recipe for _, recipe in recipes(anvil)})
        self.assertIn('"tconstruct:anvil_metal"', (MOD / 'mixins/tconstruct/source-advancement-tag.rs').read_text())

    def test_existing_native_shapes_counts_and_material_metadata(self):
        with ZipFile(MOD / 'content/reference/tconstruct-recipes.zip') as source:
            for anvil in ANVILS:
                original = json.loads(source.read('data/tconstruct/recipes/tables/' + anvil + '.json'))
                support = '#tconstruct:anvil_' + ('seared_blocks' if anvil == 'tinkers_anvil' else 'scorched_blocks')
                for variant, recipe in recipes(anvil):
                    with self.subTest(recipe=recipe):
                        native = json.loads((DATA / f'recipe/{recipe}.json').read_text())
                        self.assertEqual(native['type'], 'minecraft:crafting_shaped')
                        self.assertEqual(native['pattern'], original['pattern'])
                        self.assertEqual(native['result']['id'], original['result']['item'])
                        self.assertEqual(native['result'].get('count', 1), 1)
                        self.assertEqual(native['key']['s'], support)
                        if variant == 'mixed_alloys':
                            self.assertEqual(set(native['key']['m']), {metal_block(metal) for metal in METALS})
                            self.assertNotIn('components', native['result'])
                        else:
                            metadata = native['result']['components']['minecraft:custom_data']['tconstruct:anvil']
                            self.assertEqual(metadata['Variant'], variant)
                            self.assertEqual(native['result']['components']['minecraft:item_model'], 'tconstruct:' + recipe)
                            if variant.startswith('compat_'):
                                self.assertEqual(native['key']['m'], 'tconstruct:fake_storage_block')
                                self.assertEqual(metadata['RequiredMaterial'], 'tconstruct:' + variant.removeprefix('compat_'))
                                self.assertEqual(metadata['Metal'], metadata['RequiredMaterial'])
                            else:
                                self.assertEqual(native['key']['m'], metal_block(variant))
                                self.assertEqual(metadata['Metal'], metal_block(variant))
                                self.assertNotIn('RequiredMaterial', metadata)
        guard = (MOD / 'mixins/tconstruct/anvil-material-recipe.rs').read_text()
        self.assertIn('RequiredMaterial', guard)
        self.assertIn('fake.len() != 3', guard)

    def test_package_contains_both_unlocks_and_all_thirty_six_recipes(self):
        with tempfile.TemporaryDirectory() as tmp:
            output = Path(tmp) / 'tconstruct.hmod'
            package(output)
            with ZipFile(output) as archive:
                for anvil in ANVILS:
                    paths = [f'advancement/recipes/decorations/{anvil}.json']
                    paths.extend(f'recipe/{recipe}.json' for _, recipe in recipes(anvil))
                    for relative in paths:
                        self.assertEqual(archive.read('data/tconstruct/' + relative), (DATA / relative).read_bytes())


if __name__ == '__main__':
    unittest.main()
