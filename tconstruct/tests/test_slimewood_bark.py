"""Source parity of slimewood log-to-wood crafting."""
import json
from pathlib import Path
import unittest
from zipfile import ZipFile

ROOT = Path(__file__).resolve().parents[1]
DATA = ROOT / 'content/data/tconstruct'

class SlimewoodBark(unittest.TestCase):
    def test_all_source_bark_recipes(self):
        with ZipFile(ROOT / 'content/reference/tconstruct-recipes.zip') as source:
            for wood in ('greenheart', 'skyroot', 'bloodshroom', 'enderbark'):
                for prefix in ('', 'stripped_'):
                    with self.subTest(wood=wood, prefix=prefix):
                        name = 'stripped_log_to_wood' if prefix else 'log_to_wood'
                        original = json.loads(source.read(f'data/tconstruct/recipes/world/wood/{wood}/{name}.json'))
                        expected = dict(original)
                        expected.pop('show_notification')
                        expected['key'] = {k: v['item'] for k, v in original['key'].items()}
                        expected['result'] = {'id': original['result']['item'], 'count': original['result']['count']}
                        actual = json.loads((DATA / f'recipe/{prefix}{wood}_wood.json').read_text())
                        self.assertEqual(actual, expected)
                        self.assertEqual(actual['pattern'], ['##', '##'])
                        self.assertEqual(actual['result']['count'], 3)
                        for item in (*actual['key'].values(), actual['result']['id']):
                            self.assertTrue((DATA / f'horizon/block/{item.split(":", 1)[1]}.json').is_file(), item)

if __name__ == '__main__':
    unittest.main()
