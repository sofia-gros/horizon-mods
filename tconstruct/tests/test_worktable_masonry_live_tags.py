"""Worktable masonry follows the source tag, preserving native rock variants."""
import copy
import hashlib
import json
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]
DATA = ROOT / 'content/data/tconstruct'
FIXTURES = ROOT / 'tests/fixtures/worktable_masonry_tags'


class WorktableMasonryLiveTags(unittest.TestCase):
    def test_source_identity_and_all_unchanged_native_fields(self):
        manifest = json.loads((FIXTURES / 'manifest.json').read_text())
        raw = (FIXTURES / 'modifier_worktable.json').read_bytes()
        self.assertEqual(hashlib.sha256(raw).hexdigest(), manifest['sha256'])
        self.assertIn('/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/', manifest['source'])
        self.assertEqual(manifest['base_commit'], '9a0bc037f9fa66840e73ceffd4e24fc3014d4bf4')
        source = json.loads(raw)
        expected = manifest['recipe_without_masonry_sha256']
        self.assertEqual(len(expected), 13)
        native = {p.name: json.loads(p.read_text()) for p in (DATA / 'recipe').glob('modifier_worktable_*.json')}
        self.assertEqual(set(native), set(expected))
        for name, recipe in native.items():
            with self.subTest(recipe=name):
                self.assertEqual(recipe['key']['s'], '#' + source['key']['s']['tag'])
                self.assertEqual(recipe['pattern'], source['pattern'])
                self.assertEqual(recipe['result']['id'], source['result']['item'])
                self.assertEqual(''.join(recipe['pattern']).count('s'), 3)
                self.assertEqual(''.join(recipe['pattern']).count('r'), 4)
                unchanged = copy.deepcopy(recipe)
                del unchanged['key']['s']
                actual = hashlib.sha256(json.dumps(unchanged, sort_keys=True, separators=(',', ':')).encode()).hexdigest()
                self.assertEqual(actual, expected[name])
                self.assertNotIn('anvil_seared_blocks', json.dumps(recipe))

    def test_source_tag_is_shared_and_generator_does_not_restore_private_copy(self):
        source = json.loads((FIXTURES / 'modifier_worktable.json').read_text())
        self.assertEqual(source['key']['s'], {'tag': 'tconstruct:seared_blocks'})
        tag = json.loads((DATA / 'tags/item/seared_blocks.json').read_text())
        self.assertIn('#tconstruct:seared_bricks', tag['values'])
        self.assertNotIn('replace', tag)
        generator = (ROOT / 'tools/generate_worktable_variants.py').read_text()
        self.assertNotIn('anvil_seared_blocks', generator)
        self.assertIn("'s': '#tconstruct:seared_blocks'", generator)
        # Shared Anvil tag cases cover empty/replaced parent and child tags.
        self.assertTrue((ROOT / 'tests/test_anvil_masonry_live_tags.py').is_file())

    def test_existing_discovery_still_reaches_exactly_these_thirteen_variants(self):
        manifest = json.loads((FIXTURES / 'manifest.json').read_text())
        expected = {'tconstruct:' + Path(name).stem for name in manifest['recipe_without_masonry_sha256']}
        goal = json.loads((DATA / 'advancement/recipes/decorations/modifier_worktable.json').read_text())
        self.assertEqual(set(goal['rewards']['recipes']), expected)
        self.assertEqual(len(goal['rewards']['recipes']), 13)
        self.assertEqual(goal['criteria']['has_item']['conditions']['items'], [{'items': '#tconstruct:seared_blocks'}])


if __name__ == '__main__':
    unittest.main()
