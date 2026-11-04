"""Check geode steel shard cooking against pinned TConstruct recipes."""
import json
from pathlib import Path
import unittest
from zipfile import ZipFile

ROOT = Path(__file__).resolve().parents[1]
DATA = ROOT / 'content/data/tconstruct'

class SteelShardCooking(unittest.TestCase):
    def test_source_recipes_and_geode_loot(self):
        with ZipFile(ROOT / 'content/reference/tconstruct-recipes.zip') as source:
            for kind, ticks in [('smelting', 50), ('blasting', 25)]:
                with self.subTest(kind=kind):
                    name = f'steel_nugget_{kind}'
                    original = json.loads(source.read(f'data/tconstruct/recipes/common/materials/{name}.json'))
                    expected = dict(original)
                    expected['ingredient'] = original['ingredient']['item']
                    expected['result'] = {'id': original['result']}
                    actual = json.loads((DATA / f'recipe/{name}.json').read_text())
                    self.assertEqual(actual, expected)
                    self.assertEqual(actual['type'], f'minecraft:{kind}')
                    self.assertEqual((actual['cookingtime'], actual['experience']), (ticks, 0.2))
                    self.assertEqual((actual['ingredient'], actual['result']['id']), ('tconstruct:steel_shard', 'tconstruct:steel_nugget'))
        loot = (DATA / 'loot_table/blocks/steel_cluster.json').read_text()
        self.assertIn('tconstruct:steel_shard', loot)
        resources = (ROOT / 'tools/resources.py').read_text()
        self.assertIn("'tconstruct:steel_shard'", resources)
        self.assertIn("'tconstruct:steel_nugget'", resources)

if __name__ == '__main__':
    unittest.main()
