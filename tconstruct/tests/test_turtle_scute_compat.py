"""The compatibility boundary changes native names, not source recipe semantics."""
import json
import tomllib
from pathlib import Path
import unittest
import zipfile

ROOT = Path(__file__).resolve().parents[1]
MIXINS = ROOT / 'mixins/tconstruct'

class TurtleScuteCompat(unittest.TestCase):
    def test_pinned_recipe_and_material_semantics(self):
        with zipfile.ZipFile(ROOT / 'content/reference/tconstruct-recipes.zip') as archive:
            material = json.loads(archive.read('data/tconstruct/recipes/tools/materials/turtle_scute.json'))
            recycling = json.loads(archive.read('data/tconstruct/recipes/tables/recycling/turtle_helmet.json'))
        self.assertEqual(material, {'type': 'tconstruct:material', 'ingredient': {'item': 'minecraft:scute'}, 'material': 'tconstruct:turtle', 'needed': 1, 'value': 1})
        self.assertEqual(recycling, {'type': 'tconstruct:part_builder_recycling', 'pattern': {'item': 'tconstruct:venom_bottle'}, 'tool': {'item': 'minecraft:turtle_helmet'}, 'results': {'tconstruct:scale': {'item': 'minecraft:scute', 'count': 5}}})
        with zipfile.ZipFile(ROOT / 'content/reference/tconstruct-materials.zip') as archive:
            definition = json.loads(archive.read('data/tconstruct/tinkering/materials/definition/turtle.json'))
            stats = json.loads(archive.read('data/tconstruct/tinkering/materials/stats/turtle.json'))
        self.assertTrue(definition['craftable'])
        self.assertIn('tconstruct:maille', stats['stats'])
        self.assertIn('tconstruct:shell', stats['stats'])

    def test_both_preparation_paths_and_cache_migration(self):
        manual = (MIXINS / 'prepare.rs').read_text()
        bundled = (MIXINS / 'resource-tables.rs').read_text()
        self.assertIn('tconstruct_source_scute_compat::prepare(&original_pack, &items)?', manual)
        self.assertIn('tconstruct_source_scute_compat::prepare(&original_out, &item_table)?', bundled)
        self.assertIn('tconstruct-native-creative-v8-scute-v1', bundled)
        self.assertLess(bundled.index('tconstruct_source_scute_compat::prepare'), bundled.rindex('std::fs::write(&marker'))
        self.assertIn('source = "source-scute-compat.rs"', (MIXINS / 'horizon-mixin.toml').read_text())

    def test_shared_host_owner_and_ci_guards(self):
        manifest = tomllib.loads((MIXINS / 'horizon-mixin.toml').read_text())
        modules = [module for module in manifest['module'] if module['source'] == 'source-scute-compat.rs']
        self.assertEqual(len(modules), 1)
        self.assertEqual(modules[0]['file'], 'crates/horizon-client/src/client_lib.rs')
        self.assertEqual(modules[0]['visibility'], 'pub')
        transaction = (MIXINS / 'transaction.rs').read_text()
        self.assertIn('use crate::tconstruct_source_scute_compat as compat;', transaction)
        self.assertNotIn('crate::entry::', transaction)
        workflow = (ROOT.parent / '.github/workflows/mixins.yml').read_text()
        self.assertIn('name: Apply Mixins and check client\n        shell: bash', workflow)
        test = 'tconstruct_tx::tests::native_turtle_scute_enters_source_material_lookup_and_recycling_output'
        for owner in ('game::', ''):
            self.assertIn(f'test {owner}{test} ... ok', workflow)
        self.assertIn('-p horizon-client-lib --lib --no-default-features --profile fast tconstruct_tx::tests', workflow)
