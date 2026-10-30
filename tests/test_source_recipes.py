"""The shipped source recipe data must match the pinned reference catalog."""
import gzip
import hashlib
import zipfile
import json
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]

class SourceRecipes(unittest.TestCase):
    def test_original_generated_recipe_pack(self):
        pack = ROOT / 'content/reference/tconstruct-recipes.zip'
        self.assertEqual(hashlib.sha256(pack.read_bytes()).hexdigest(), 'd84f5da24c464af5d6b158ddf324c0de0f2b27a5cdeb8737cace6d6ab4699951')
        with gzip.open(ROOT / 'docs/upstream-recipes.json.gz', 'rt', encoding='utf-8') as f:
            catalog = json.load(f)
        entries = catalog['groups']['part_builder'] + [e for e in catalog['groups']['materials'] if e['data'].get('type') == 'tconstruct:material']
        with zipfile.ZipFile(pack) as archive:
            self.assertEqual(len(archive.namelist()), 3067)
            for entry in entries:
                name = entry['source'].removeprefix('src/generated/resources/')
                with self.subTest(source=name):
                    self.assertEqual(json.loads(archive.read(name)), entry['data'])

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
