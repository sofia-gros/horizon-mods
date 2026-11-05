"""Source conversion identity and bounded native crafting projection wiring."""
import hashlib
import json
from pathlib import Path
import tomllib
import unittest

ROOT = Path(__file__).resolve().parents[1]
MIXINS = ROOT / 'mixins/tconstruct'
FIXTURES = ROOT / 'tests/fixtures/fake_metal_crafting'


class FakeMetalCrafting(unittest.TestCase):
    def test_exact_source_conversion_fixtures_and_native_unit_counts(self):
        manifest = json.loads((FIXTURES / 'manifest.json').read_text())
        self.assertEqual(set(manifest), {'fake_ingot_to_block.json', 'fake_block_to_ingots.json'})
        for name, entry in manifest.items():
            raw = (FIXTURES / name).read_bytes()
            self.assertEqual(hashlib.sha256(raw).hexdigest(), entry['sha256'])
            self.assertIn('/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/', entry['source'])
        block = json.loads((FIXTURES / 'fake_ingot_to_block.json').read_text())
        self.assertEqual(block['pattern'], ['###', '###', '###'])
        self.assertEqual(block['result'], {'item': 'tconstruct:fake_storage_block'})
        self.assertEqual(block['key']['#']['material'], {'type': 'tconstruct:has_part', 'part': 'tconstruct:fake_storage_block'})
        ingots = json.loads((FIXTURES / 'fake_block_to_ingots.json').read_text())
        self.assertEqual(ingots['result'], {'count': 9, 'item': 'tconstruct:fake_ingot'})
        self.assertEqual(ingots['ingredients'][0]['material'], {'type': 'tconstruct:has_part', 'part': 'tconstruct:fake_ingot'})

    def test_modules_and_read_only_guard_are_connected(self):
        manifest = tomllib.loads((MIXINS / 'horizon-mixin.toml').read_text())
        modules = {entry['id']: entry for entry in manifest['module']}
        self.assertEqual(modules['source-fake-material-crafting']['source'], 'source-fake-material-crafting.rs')
        self.assertEqual(modules['source-fake-material-crafting']['file'], 'crates/horizon-client/src/crafting.rs')
        self.assertEqual(modules['source-fake-material-world']['file'], 'crates/horizon-client/src/game.rs')
        hook = next(entry for entry in manifest['injection'] if entry['id'] == 'fake-material-recipe')
        self.assertEqual(hook['target'], 'horizon_client_lib::crafting::Recipes::assemble_if_matches')
        self.assertEqual(hook['kind'], 'HEAD')
        self.assertTrue(hook['cancellable'])
        guard = (MIXINS / hook['helper']).read_text()
        self.assertIn('tconstruct_source_fake_material_crafting::plan', guard)
        self.assertIn('planned.same_item_same_components(&output)', guard)
        self.assertIn('input.width, input.height', guard)
        self.assertIn('ingredient.custom', guard)
        self.assertNotIn('data_mut()', guard)
        self.assertNotIn('Break(Some', guard)

    def test_refresh_uses_complete_world_snapshot_and_removes_stale_projection(self):
        reload = (MIXINS / 'recipe-reload.rs').read_text()
        call = 'crate::game::tconstruct_source_fake_material_world::refresh(game);'
        self.assertIn(call, reload)
        self.assertGreater(reload.index(call), reload.index('__horizon_mixins_tconstruct_recipe_snapshot_mut() = Some(recipes)'))
        world = (MIXINS / 'source-fake-material-world.rs').read_text()
        for expected in ('game.world_tables.as_ref()', 'native.or_else(|| recipes.get(source_id))',
                         'fake_material_can_use(', 'materials.len() > 512', 'value == expected',
                         'generated(game).unwrap_or_default()', '!id.starts_with(PREFIX)'):
            self.assertIn(expected, world)
        self.assertNotIn('static mut', world)
        helper = (MIXINS / 'source-builder-world.rs').read_text()
        self.assertIn('fake_material_can_use(world, items, material, "tconstruct:fake_storage_block")', helper)
        self.assertIn('source_supported_fake_items(base, material_tags, &values, true)?.contains(output)', helper)


if __name__ == '__main__':
    unittest.main()
