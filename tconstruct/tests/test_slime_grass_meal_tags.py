"""The growth algorithm uses the pinned upstream grass tag hierarchy."""
import json
from pathlib import Path
import unittest
import zipfile

ROOT = Path(__file__).resolve().parents[1]

class SlimeGrassMealTags(unittest.TestCase):
    def test_tags_match_pinned_reference(self):
        with zipfile.ZipFile(ROOT / 'content/reference/tconstruct-block-tags.zip') as archive:
            for suffix in ('', '/earth', '/sky', '/ender'):
                expected = json.loads(archive.read(f'data/tconstruct/tags/blocks/slimy_grass{suffix}.json'))
                actual = json.loads((ROOT / f'content/data/tconstruct/tags/block/slimy_grass{suffix}.json').read_text())
                self.assertEqual(actual, expected)

if __name__ == '__main__':
    unittest.main()
