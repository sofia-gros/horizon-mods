"""Pinned Multishot source recipe, definition and actual hook target contracts."""
import json
from pathlib import Path
import re
import tomllib
import unittest
from zipfile import ZipFile

ROOT = Path(__file__).resolve().parents[1]
MIXINS = ROOT / 'mixins/tconstruct'


class Multishot(unittest.TestCase):
    def test_station_recipe_is_the_pinned_source_recipe(self):
        with ZipFile(ROOT / 'content/reference/tconstruct-recipes.zip') as archive:
            source = json.loads(archive.read('data/tconstruct/recipes/tools/modifiers/ability/multishot.json'))
        text = (MIXINS / 'source-upgrade-rules.rs').read_text()
        fixture = re.search(r'fn multishot_recipe.*?let recipe = br#"(.*?)"#;', text, re.S).group(1)
        self.assertEqual(json.loads(fixture), source)
        self.assertEqual(source['slots'], {'abilities': 1})

    def test_definition_is_pinned_and_injection_owns_ammunition(self):
        with ZipFile(ROOT / 'content/reference/tconstruct-modifiers.zip') as archive:
            source = json.loads(archive.read('data/tconstruct/tinkering/modifiers/multishot.json'))
        helper = (MIXINS / 'source-multishot.rs').read_text()
        fixture = re.search(r'let source=br#"(.*?)"#;', helper).group(1)
        self.assertEqual(json.loads(fixture), source)
        manifest = tomllib.loads((MIXINS / 'horizon-mixin.toml').read_text())
        hook = next(v for v in manifest['injection'] if v['id'] == 'source-multishot-loading')
        self.assertEqual(hook['target'], "horizon_client::game::crossbow::Game<'_>::tick_crossbow")
        self.assertEqual(hook['kind'], 'HEAD')
        self.assertTrue(hook['cancellable'])
        runtime = (MIXINS / hook['helper']).read_text()
        self.assertLess(runtime.index('plan.consumed.iter().any'), runtime.index('for (i, _, count) in plan.consumed'))
        self.assertIn('game.remote.is_some()', runtime)
        self.assertIn('!source::known_definition(definition)', runtime)
        station = (MIXINS / 'source-station-world.rs').read_text()
        self.assertIn('resolved_traits.as_ref()?;', station)
        self.assertIn('tconstruct_multishot::known_definition(definition)', station)


if __name__ == '__main__':
    unittest.main()
