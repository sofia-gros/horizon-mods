"""Anvil recipes follow source masonry tags without changing metal/results."""
import copy
import hashlib
import json
from pathlib import Path
import sys
import tempfile
import unittest
from zipfile import ZipFile

ROOT = Path(__file__).resolve().parents[1]
DATA = ROOT / 'content/data/tconstruct'
FIXTURES = ROOT / 'tests/fixtures/anvil_masonry_tags'
sys.path.insert(0, str(ROOT / 'tools'))
from package_hmod import package


def fingerprint_without_masonry(recipe):
    value = copy.deepcopy(recipe)
    del value['key']['s']
    return hashlib.sha256(json.dumps(value, sort_keys=True, separators=(',', ':')).encode()).hexdigest()


def expand_tag(name, overrides=None):
    """Independent resource projection, not a live host matcher assertion."""
    overrides = overrides or {}
    document = overrides.get(name)
    if document is None:
        document = json.loads((DATA / 'tags/item' / (name.split(':', 1)[1] + '.json')).read_text())
    members = set()
    for value in document['values']:
        if value.startswith('#'):
            members.update(expand_tag(value[1:], overrides))
        else:
            members.add(value)
    return members


class AnvilMasonryLiveTags(unittest.TestCase):
    def test_exact_pinned_sources_and_native_tag_graph(self):
        manifest = json.loads((FIXTURES / 'manifest.json').read_text())
        self.assertEqual(set(manifest), {'recipes/tinkers_anvil.json', 'recipes/scorched_anvil.json',
            'tags/seared_blocks.json', 'tags/scorched_blocks.json', 'tags/seared_bricks.json'})
        for relative, entry in manifest.items():
            raw = (FIXTURES / relative).read_bytes()
            self.assertEqual(hashlib.sha256(raw).hexdigest(), entry['sha256'])
            self.assertIn('/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/', entry['source'])
            if relative.startswith('tags/'):
                actual = json.loads((DATA / 'tags/item' / Path(relative).name).read_text())
                self.assertEqual(actual, json.loads(raw))
                self.assertNotIn('replace', actual)

    def test_all_native_variants_change_only_the_masonry_tag(self):
        baseline = json.loads((FIXTURES / 'native_unchanged_fields.json').read_text())
        self.assertEqual(baseline['base_commit'], '47dc59a752e5be68040bf1b465d53fb019cfa3a4')
        expected = baseline['recipe_without_masonry_sha256']
        self.assertEqual(len(expected), 36)
        recipes = {path.name: json.loads(path.read_text()) for path in (DATA / 'recipe').glob('*anvil*.json')}
        self.assertEqual(set(recipes), set(expected))
        for name, recipe in recipes.items():
            with self.subTest(recipe=name):
                family = 'scorched_anvil' if 'scorched_anvil' in name else 'tinkers_anvil'
                source = json.loads((FIXTURES / 'recipes' / (family + '.json')).read_text())
                self.assertEqual(recipe['key']['s'], '#' + source['key']['s']['tag'])
                self.assertEqual(recipe['pattern'], source['pattern'])
                self.assertEqual(recipe['type'], 'minecraft:crafting_shaped')
                self.assertEqual(recipe['result']['id'], source['result']['item'])
                self.assertEqual(fingerprint_without_masonry(recipe), expected[name])
                self.assertEqual(''.join(recipe['pattern']).count('m'), 3)
                self.assertEqual(''.join(recipe['pattern']).count('s'), 4)
                self.assertNotIn('anvil_seared_blocks', json.dumps(recipe))
                self.assertNotIn('anvil_scorched_blocks', json.dumps(recipe))

    def test_defaults_match_old_lists_but_live_replacements_do_not_fall_back(self):
        for family, count in [('seared', 7), ('scorched', 5)]:
            live = 'tconstruct:' + family + '_blocks'
            legacy = 'tconstruct:anvil_' + family + '_blocks'
            self.assertEqual(expand_tag(live), expand_tag(legacy))
            self.assertEqual(len(expand_tag(live)), count)
            self.assertEqual(expand_tag(live, {live: {'replace': True, 'values': []}}), set())
            changed = {live: {'replace': True, 'values': ['minecraft:stone']}}
            self.assertEqual(expand_tag(live, changed), {'minecraft:stone'})
            self.assertNotEqual(expand_tag(legacy, changed), {'minecraft:stone'})
        without_bricks = {'tconstruct:seared_bricks': {'replace': True, 'values': []}}
        self.assertEqual(expand_tag('tconstruct:seared_blocks', without_bricks), {
            'tconstruct:seared_stone', 'tconstruct:seared_cracked_bricks',
            'tconstruct:seared_cobble', 'tconstruct:seared_paver'})

    def test_generator_keeps_source_tags_and_discovery_targets_existing_recipes(self):
        generator = (ROOT / 'tools/generate_anvil_variants.py').read_text()
        self.assertNotIn('anvil_seared_blocks', generator)
        self.assertNotIn('anvil_scorched_blocks', generator)
        self.assertIn("value['key']['s']='#tconstruct:'", generator)
        for family in ('tinkers_anvil', 'scorched_anvil'):
            goal = json.loads((DATA / 'advancement/recipes/decorations' / (family + '.json')).read_text())
            for recipe in goal['rewards']['recipes']:
                self.assertTrue((DATA / 'recipe' / (recipe.split(':', 1)[1] + '.json')).is_file())

    def test_live_masonry_tag_and_every_recipe_are_packaged(self):
        baseline = json.loads((FIXTURES / 'native_unchanged_fields.json').read_text())
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / 'tconstruct.hmod'
            package(output)
            with ZipFile(output) as archive:
                paths = ['tags/item/seared_blocks.json'] + [
                    'recipe/' + name for name in baseline['recipe_without_masonry_sha256']]
                for path in paths:
                    self.assertEqual(archive.read('data/tconstruct/' + path), (DATA / path).read_bytes())


if __name__ == '__main__':
    unittest.main()
