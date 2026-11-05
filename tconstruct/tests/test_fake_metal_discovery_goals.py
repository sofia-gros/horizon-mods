"""Pinned source goals, world-local dynamic mapping and runtime hook wiring."""
import hashlib
import json
from pathlib import Path
import tomllib
import unittest

ROOT = Path(__file__).resolve().parents[1]
MIXINS = ROOT / 'mixins/tconstruct'
FIXTURES = ROOT / 'tests/fixtures/fake_metal_discovery'
GOALS = ROOT / 'content/data/tconstruct/advancement/recipes/misc/tools/parts'


class FakeMetalDiscoveryGoals(unittest.TestCase):
    def test_native_goals_preserve_source_except_recipe_predicate_spelling(self):
        manifest = json.loads((FIXTURES / 'manifest.json').read_text())
        self.assertEqual(set(manifest), {'fake_ingot_to_block.json', 'fake_block_to_ingots.json'})
        for name, item in [('fake_ingot_to_block.json', 'tconstruct:fake_ingot'),
                           ('fake_block_to_ingots.json', 'tconstruct:fake_storage_block')]:
            raw = (FIXTURES / name).read_bytes()
            self.assertEqual(hashlib.sha256(raw).hexdigest(), manifest[name]['sha256'])
            self.assertIn('/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/', manifest[name]['source'])
            expected = json.loads(raw)
            condition = expected['criteria']['has_the_recipe']['conditions']
            condition['recipes'] = condition.pop('recipe')
            actual = json.loads((GOALS / name).read_text())
            self.assertEqual(actual, expected)
            self.assertEqual(actual['requirements'], [['has_item', 'has_the_recipe']])
            self.assertEqual(actual['criteria']['has_item']['conditions']['items'], [{'items': [item]}])

    def test_tree_module_and_initial_prefetch_hook_are_connected(self):
        manifest = tomllib.loads((MIXINS / 'horizon-mixin.toml').read_text())
        modules = {m['id']: m for m in manifest['module']}
        self.assertEqual(modules['source-fake-material-discovery']['file'],
                         'crates/horizon-client/src/advancements.rs')
        self.assertEqual(modules['source-fake-material-discovery']['visibility'], 'pub')
        self.assertEqual(modules['source-fake-material-discovery-world']['file'],
                         'crates/horizon-client/src/game.rs')
        hook = next(h for h in manifest['injection'] if h['id'] == 'fake-material-discovery-tree')
        self.assertEqual(hook['target'], 'horizon_client::game::advancements::Game<\'_>::advancement_tree')
        self.assertEqual(hook['kind'], 'RETURN')
        self.assertIn('tconstruct_source_fake_material_discovery_world::apply', (MIXINS / hook['helper']).read_text())

    def test_only_current_index_variants_are_mapped_after_projection(self):
        world = (MIXINS / 'source-fake-material-discovery-world.rs').read_text()
        for expected in ('game.remote.is_some()', '__horizon_mixins_tconstruct_recipe_snapshot().is_some()',
                         'tables.stations.recipe_index.ids()', 'Arc::ptr_eq(previous, &tree)',
                         'tconstruct_invalidate_advancement_inventory_index()'):
            self.assertIn(expected, world)
        projection = (MIXINS / 'source-fake-material-world.rs').read_text()
        self.assertLess(projection.index('tconstruct_source_fake_material_book::replace('),
                        projection.index('tconstruct_source_fake_material_discovery_world::refresh(game)'))
        mapping = (MIXINS / 'source-fake-material-discovery.rs').read_text()
        for expected in ('minecraft:impossible', 'old_shape != source(family)',
                         'old_shape == mapped(family, &old.rewards.recipes)', 'next.index_recipes()',
                         'list: tree.list.clone()', 'ids.len() <= 512', 'has_native_recipe_{hex}'):
            self.assertIn(expected, mapping)
        for forbidden in ('add_recipes(', 'static mut', 'award('):
            self.assertNotIn(forbidden, mapping)
            self.assertNotIn(forbidden, world)


if __name__ == '__main__':
    unittest.main()
