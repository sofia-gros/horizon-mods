"""Check knightmetal raw-nugget cooking against pinned source data."""
import json
from pathlib import Path
import unittest
from zipfile import ZipFile

ROOT = Path(__file__).resolve().parents[1]
DATA = ROOT / 'content/data'

class KnightmetalShardCooking(unittest.TestCase):
    def test_source_tag_recipes_and_geode_loot(self):
        reference = json.loads((ROOT / 'content/reference/tconstruct-tags/data/forge/tags/items/raw_nuggets/knightmetal.json').read_text())
        native = json.loads((DATA / 'forge/tags/item/raw_nuggets/knightmetal.json').read_text())
        self.assertEqual(native, reference)
        self.assertFalse(native.get('replace', False))
        self.assertEqual(native['values'], ['tconstruct:knightmetal_shard', {'id': 'twilightforest:armor_shard', 'required': False}])
        with ZipFile(ROOT / 'content/reference/tconstruct-recipes.zip') as source:
            for kind, ticks in [('smelting', 50), ('blasting', 25)]:
                with self.subTest(kind=kind):
                    name = f'knightmetal_nugget_{kind}'
                    original = json.loads(source.read(f'data/tconstruct/recipes/common/materials/{name}.json'))
                    expected = dict(original)
                    expected['ingredient'] = '#' + original['ingredient']['tag']
                    expected['result'] = {'id': original['result']}
                    actual = json.loads((DATA / f'tconstruct/recipe/{name}.json').read_text())
                    self.assertEqual(actual, expected)
                    self.assertEqual(actual['type'], f'minecraft:{kind}')
                    self.assertEqual((actual['cookingtime'], actual['experience']), (ticks, 0.2))
                    self.assertEqual((actual['ingredient'], actual['result']['id']), ('#forge:raw_nuggets/knightmetal', 'tconstruct:knightmetal_nugget'))
        loot = (DATA / 'tconstruct/loot_table/blocks/knightmetal_cluster.json').read_text()
        self.assertIn('tconstruct:knightmetal_shard', loot)
        resources = (ROOT / 'tools/resources.py').read_text()
        self.assertIn("'tconstruct:knightmetal_shard'", resources)
        self.assertIn("'tconstruct:knightmetal_nugget'", resources)

if __name__ == '__main__':
    unittest.main()
