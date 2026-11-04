"""Check decorative smeltery recipes against pinned source data."""
import json
from pathlib import Path
import unittest
from zipfile import ZipFile

ROOT = Path(__file__).resolve().parents[1]
DATA = ROOT / 'content/data/tconstruct'

class SmelteryDecorativeRecipes(unittest.TestCase):
    def test_source_recipes_and_blocks(self):
        cases = [
            ('seared', 'seared_fancy_bricks_crafting', 'seared_bricks_slab', 'seared_fancy_bricks', ['s', 's'], 1),
            ('scorched', 'chiseled_scorched_bricks_crafting', 'scorched_bricks_slab', 'chiseled_scorched_bricks', ['s', 's'], 1),
            ('scorched', 'polished_scorched_stone_crafting', 'scorched_stone', 'polished_scorched_stone', ['bb', 'bb'], 4),
        ]
        with ZipFile(ROOT / 'content/reference/tconstruct-recipes.zip') as source:
            for material, name, input_block, output_block, pattern, count in cases:
                with self.subTest(name=name):
                    original = json.loads(source.read(f'data/tconstruct/recipes/smeltery/{material}/{name}.json'))
                    expected = dict(original)
                    expected.pop('show_notification')
                    expected['key'] = {k: v['item'] for k, v in original['key'].items()}
                    expected['result'] = {'id': original['result']['item'], **({'count': original['result']['count']} if 'count' in original['result'] else {})}
                    actual = json.loads((DATA / f'recipe/{name}.json').read_text())
                    self.assertEqual(actual, expected)
                    self.assertEqual(actual['pattern'], pattern)
                    self.assertEqual(list(actual['key'].values()), [f'tconstruct:{input_block}'])
                    self.assertEqual(actual['result'].get('count', 1), count)
                    self.assertEqual(actual['result']['id'], f'tconstruct:{output_block}')
                    for block in (input_block, output_block):
                        self.assertTrue((DATA / f'horizon/block/{block}.json').is_file())

if __name__ == '__main__':
    unittest.main()
