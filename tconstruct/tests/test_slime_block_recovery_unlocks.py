"""Pinned colored slime block recovery discovery; no physics assertions."""
import hashlib
import json
from pathlib import Path
import sys
import tempfile
import unittest
from zipfile import ZipFile

MOD = Path(__file__).resolve().parents[1]
DATA = MOD / 'content/data/tconstruct'
FIXTURES = MOD / 'tests/fixtures/slime_block_recovery_unlocks'
KINDS = ('sky', 'ichor', 'ender')
sys.path.insert(0, str(MOD / 'tools'))
from package_hmod import package


class SlimeBlockRecoveryUnlocks(unittest.TestCase):
    def test_source_item_criteria_and_or_recipe_rewards(self):
        hashes = json.loads((FIXTURES / 'sha256.json').read_text())
        self.assertEqual(set(hashes), set(KINDS))
        for kind in KINDS:
            with self.subTest(kind=kind):
                name = kind + '_slimeball_from_block'
                raw = (FIXTURES / (kind + '.json')).read_bytes()
                self.assertEqual(hashlib.sha256(raw).hexdigest(), hashes[kind])
                expected = json.loads(raw)
                self.assertEqual(expected['criteria']['has_the_recipe']['conditions'],
                                 {'recipe': f'tconstruct:common/slime/{kind}/slimeball_from_block'})
                expected['criteria']['has_the_recipe']['conditions'] = {'recipes': 'tconstruct:' + name}
                expected['rewards']['recipes'] = ['tconstruct:' + name]
                native = json.loads((DATA / f'advancement/recipes/misc/{name}.json').read_text())
                self.assertEqual(native, expected)
                self.assertEqual(native['criteria']['has_item']['conditions'],
                                 {'items': [{'items': ['tconstruct:' + kind + '_slime']}]})
                self.assertEqual(native['requirements'], [['has_item', 'has_the_recipe']])
                self.assertEqual(native['parent'], 'minecraft:recipes/root')
                self.assertNotIn('display', native)

    def test_rewards_reference_existing_same_color_nine_ball_recipes(self):
        for kind in KINDS:
            recipe = json.loads((DATA / f'recipe/{kind}_slimeball_from_block.json').read_text())
            self.assertEqual(recipe['type'], 'minecraft:crafting_shapeless')
            self.assertEqual(recipe['ingredients'], ['tconstruct:' + kind + '_slime'])
            self.assertEqual(recipe['result'], {'id': 'tconstruct:' + kind + '_slime_ball', 'count': 9})
            self.assertTrue((DATA / f'horizon/block/{kind}_slime.json').is_file())

    def test_package_contains_all_three_advancements_and_recipes(self):
        with tempfile.TemporaryDirectory() as tmp:
            output = Path(tmp) / 'tconstruct.hmod'
            package(output)
            with ZipFile(output) as archive:
                for kind in KINDS:
                    name = kind + '_slimeball_from_block'
                    for relative in (f'advancement/recipes/misc/{name}.json', f'recipe/{name}.json'):
                        self.assertEqual(archive.read('data/tconstruct/' + relative), (DATA / relative).read_bytes())


if __name__ == '__main__':
    unittest.main()
