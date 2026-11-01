"""Check the playable anvil recipes against the pinned source ingredients."""
import json
from pathlib import Path
import unittest
import zipfile

ROOT = Path(__file__).resolve().parents[1]
DATA = ROOT / 'content/data/tconstruct'

class AnvilRecipes(unittest.TestCase):
    def test_source_pattern_and_registered_ingredients(self):
        with zipfile.ZipFile(ROOT / 'content/reference/tconstruct-recipes.zip') as archive:
            for anvil, material in [('tinkers_anvil', 'seared'), ('scorched_anvil', 'scorched')]:
                source = json.loads(archive.read(f'data/tconstruct/recipes/tables/{anvil}.json'))
                recipes = sorted((DATA / 'recipe').glob(f'{anvil}_*.json'))
                self.assertGreater(len(recipes), 0)
                tag = json.loads((DATA / 'tags/item' / f'anvil_{material}_blocks.json').read_text())['values']
                self.assertTrue(tag)
                for block in tag:
                    self.assertTrue((DATA / 'horizon/block' / (block.split(':')[1] + '.json')).is_file())
                for path in recipes:
                    recipe = json.loads(path.read_text())
                    self.assertEqual(recipe['pattern'], source['pattern'])
                    self.assertEqual(recipe['result']['id'], f'tconstruct:{anvil}')
                    self.assertEqual(recipe['key']['s'], f'#tconstruct:anvil_{material}_blocks')
                    components = recipe['result']['components']
                    variant = path.stem
                    self.assertEqual(components['minecraft:item_model'], f'tconstruct:{variant}')
                    visual = components['minecraft:custom_data']['tconstruct:anvil']
                    self.assertEqual(visual['Metal'], variant.removeprefix(anvil + '_'))
                    item = json.loads((ROOT / 'content/assets/tconstruct/items' / f'{variant}.json').read_text())
                    self.assertEqual(item['model']['model'], f'tconstruct:item/{variant}')
                    model = json.loads((ROOT / 'content/assets/tconstruct/models/block/table' / f'{variant}.json').read_text())
                    self.assertEqual(model['textures']['top'], visual['Texture'])
                    metal = recipe['key']['m']
                    if metal.startswith('tconstruct:'):
                        self.assertTrue((DATA / 'horizon/block' / (metal.split(':')[1] + '.json')).is_file())
                    else:
                        self.assertEqual(metal, 'minecraft:netherite_block')

if __name__ == '__main__':
    unittest.main()
