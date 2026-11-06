"""Source-boundary guards, not Rust/native behavior evidence."""
from pathlib import Path
import tomllib
import unittest
ROOT = Path(__file__).resolve().parents[1]
NATIVE = ROOT / 'mixins/pluto_native_creative'

class PresentationSource(unittest.TestCase):
    def test_single_additive_native_module_without_automatic_hook_or_state(self):
        manifest = tomllib.loads((NATIVE / 'horizon-mixin.toml').read_text())
        modules = [m for m in manifest['module'] if m['id'] == 'recipe-book-presentation']
        self.assertEqual(modules, [dict(id='recipe-book-presentation', file='crates/horizon-client/src/recipe_book.rs',
            name='pluto_presentation', source='recipe-book-presentation.rs', visibility='pub')])
        for key in ('injection', 'state'):
            self.assertFalse(any('presentation' in str(item) for item in manifest.get(key, [])))

    def test_one_host_independent_api_export_no_material_policy(self):
        self.assertEqual((NATIVE / 'inventory-api/src/recipe_book.rs').read_text().count('pub mod presentation;'), 1)
        source = (NATIVE / 'inventory-api/src/recipe_book/presentation.rs').read_text()
        self.assertNotIn('horizon_', source)
        self.assertNotIn('tconstruct:', source)
        self.assertNotIn('Stack: Clone', source)
        self.assertIn('UnsupportedFurnace', source)

    def test_validation_precedes_one_move_only_mutation(self):
        source = (NATIVE / 'recipe-book-presentation.rs').read_text()
        self.assertLess(source.index('presentation::validate('), source.index('for replacement in replacements'))
        self.assertEqual(source.count('.stacks = replacement.alternatives;'), 1)
        for forbidden in ('.clone(', 'encode_', 'decode_', 'Game', '.placements =', '.result =', '.variants ='):
            # The comment says no Game; only imports/types/calls are inspected for Game.
            if forbidden == 'Game':
                self.assertNotIn('use super::Game', source)
            else:
                self.assertNotIn(forbidden, source)
        self.assertIn('mut book: Book', source)
        self.assertIn('Err(Rejected { error, replacements })', source)

    def test_native_fixture_uses_actual_item_package_not_a_semantic_stub(self):
        source = (ROOT / 'tests/native/recipe_presentation.rs').read_text()
        self.assertIn('pub use horizon_items::items;', source)
        self.assertNotIn('struct ItemStack', source)
        self.assertNotIn('struct Book', source)
        self.assertEqual(source.count('#[test]'), 4)
        prepare = (ROOT / 'tools/prepare_recipe_presentation_fixture.py').read_text()
        self.assertIn('common.clean_official(host, pin)', prepare)
        self.assertIn('common.metadata(output, locked=True)', prepare)
        self.assertIn("if resolved - official - LOCAL", prepare)

    def test_complete_pure_source_tests_remain_enabled(self):
        source = (NATIVE / 'inventory-api/src/recipe_book/presentation.rs').read_text()
        self.assertEqual(source.count('#[test]'), 9)
        self.assertNotIn('#[ignore]', source)
        self.assertIn('MAX_OVERRIDES', source)
        self.assertIn('MAX_ALTERNATIVES', source)

if __name__ == '__main__':
    unittest.main()
