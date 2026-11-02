"""Pin representative TConstruct input for the shared Creative API integration."""
import json
from pathlib import Path
import sys
import unittest

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / 'tools'))
from creative_blocks import add_block_items
from creative_materials import build_variants


class CreativeRegistrationFixture(unittest.TestCase):
    def test_fixture_matches_real_tconstruct_inputs(self):
        fixture = json.loads((ROOT / 'tests/fixtures/creative-registration/case.json').read_text())
        source = json.loads((ROOT / 'content/creative-tabs.json').read_text())
        groups = fixture['groups']
        for expected, actual in zip(groups, source, strict=True):
            for key in ('id', 'title', 'icon', 'row', 'column'):
                self.assertEqual(expected[key], actual[key])
        expanded = build_variants(
            [dict(source[0], items=['{count:1,id:"tconstruct:pickaxe"}'])],
            ROOT / 'content/reference/tconstruct-tool-definitions.zip',
            ROOT / 'content/reference/tconstruct-materials.zip',
        )[0]['items']
        self.assertIn(groups[0]['items'][1], expanded)
        blocks = add_block_items(source, ROOT / 'content/data/tconstruct/horizon/block')
        self.assertIn(groups[1]['items'][0], blocks[1]['items'])
        search = fixture['inventory']['tabs'][1]
        self.assertEqual(len(search['items']), len(search['lines']))
        self.assertEqual(len(search['items']), len(search['tags']))


if __name__ == '__main__':
    unittest.main()
