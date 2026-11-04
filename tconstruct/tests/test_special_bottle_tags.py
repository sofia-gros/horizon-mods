"""Check bundled special bottle item tags against their pinned source data."""
import json
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]

class SpecialBottleTags(unittest.TestCase):
    def test_source_tags_and_registered_items(self):
        for kind in ('splash', 'lingering'):
            with self.subTest(kind=kind):
                path = ROOT / f'content/reference/tconstruct-tags/data/forge/tags/items/bottles/{kind}.json'
                tag = json.loads(path.read_text())
                self.assertEqual(tag['values'], [f'tconstruct:{kind}_bottle'])
                item = json.loads((ROOT / f'content/data/tconstruct/horizon/item/{kind}_bottle.json').read_text())
                self.assertEqual(item['template'], 'minecraft:glass_bottle')
                self.assertTrue((ROOT / f'content/assets/tconstruct/items/{kind}_bottle.json').is_file())
                self.assertTrue((ROOT / f'content/assets/tconstruct/models/item/{kind}_bottle.json').is_file())

if __name__ == '__main__':
    unittest.main()
