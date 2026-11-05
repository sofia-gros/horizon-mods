"""Pinned discovery source, native translation and config-only contribution."""
import copy
import hashlib
import json
from pathlib import Path
import re
import unittest

ROOT = Path(__file__).resolve().parents[1]
FIXTURES = ROOT / 'tests/fixtures/configured_slime_discovery'


class ConfiguredSlimeDiscovery(unittest.TestCase):
    def test_source_criteria_are_emitted_only_with_their_configured_recipe(self):
        helper = (ROOT / 'mixins/tconstruct/source-slime-crafting.rs').read_text()
        entries = re.findall(r'\("(recipe/tconstruct:[^"]+)", "(advancement/tconstruct:[^"]+)", br##"(.*?)"##\)', helper)
        self.assertEqual(len(entries), 2)
        manifest = json.loads((FIXTURES / 'manifest.json').read_text())
        self.assertEqual(set(manifest), {'lead', 'sticky_piston'})
        groups = {'lead': 'tools', 'sticky_piston': 'redstone'}
        for recipe_path, advancement_path, payload in entries:
            name = recipe_path.rsplit('/', 1)[-1]
            source_path = f'{groups[name]}/common/slime/{name}.json'
            raw = (FIXTURES / (name + '.json')).read_bytes()
            self.assertEqual(manifest[name], {'source': source_path, 'sha256': hashlib.sha256(raw).hexdigest()})
            original = json.loads(raw)
            self.assertEqual(original['criteria']['has_slime_ball']['conditions']['items'], [{'tag': 'forge:slimeballs'}])
            expected = copy.deepcopy(original)
            expected['criteria']['has_slime_ball']['conditions']['items'] = [{'items': '#forge:slimeballs'}]
            expected['criteria']['has_the_recipe']['conditions'] = {'recipes': recipe_path.removeprefix('recipe/')}
            self.assertEqual(json.loads(payload), expected)
            self.assertEqual(advancement_path, 'advancement/tconstruct:recipes/' + source_path.removesuffix('.json'))
            self.assertEqual(expected['requirements'], [['has_slime_ball', 'has_the_recipe']])
            self.assertNotIn('display', expected)
            self.assertFalse((ROOT / 'content/data/tconstruct/advancement/recipes' / source_path).exists())
        self.assertIn('config.get("slime_recipe_fix") != Some(&true)', helper)
        self.assertIn('ADVANCEMENTS.iter().find(|entry| entry.0 == path)', helper)
        self.assertIn('existing == advancement_path', helper)


if __name__ == '__main__':
    unittest.main()
