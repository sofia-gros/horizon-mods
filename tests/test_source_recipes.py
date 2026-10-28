"""The shipped source recipe data must match the pinned reference catalog."""
import gzip
import json
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]

class SourceRecipes(unittest.TestCase):
    def test_part_and_material_inputs_match_catalog(self):
        with gzip.open(ROOT / 'docs/upstream-recipes.json.gz', 'rt', encoding='utf-8') as f:
            catalog = json.load(f)
        compat = json.loads((ROOT / 'compatibility.json').read_text())
        self.assertEqual(catalog['revision'], compat['tconstruct_revision'])
        entries = catalog['groups']['part_builder'] + [e for e in catalog['groups']['materials'] if e['data'].get('type') == 'tconstruct:material']
        self.assertEqual(len(entries), 245)
        for entry in entries:
            path = ROOT / 'content' / entry['source'].removeprefix('src/generated/resources/')
            with self.subTest(source=entry['source']):
                self.assertEqual(json.loads(path.read_text()), entry['data'])

if __name__ == '__main__':
    unittest.main()
