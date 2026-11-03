"""Keep the bounded tip-clearing route on the casting transaction."""
import pathlib
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[1] / 'mixins/tconstruct'


class TipClearingWiring(unittest.TestCase):
    def test_world_requires_effective_modifier_and_preserves_revalidation(self):
        source = (ROOT / 'source-melter-world.rs').read_text()
        self.assertIn('level_with_upgrade(stack, &traits, "tconstruct:tipped")', source)
        self.assertIn('tconstruct_source_tip_clearing::clear(stack, level)', source)
        self.assertIn('catalog.tip_clearing(basin, &stack.item, fluid', source)
        runtime = (ROOT / 'source-machine-runtime.rs').read_text()
        self.assertIn('casting_recipe_at(game, pos, kind, &casting, &casting.tank)', runtime)
        self.assertIn('stored.same_item_same_components(&result)', runtime)

    def test_tipping_recipe_not_enabled(self):
        lookup = (ROOT / 'source-melting-recipes.rs').read_text()
        self.assertNotIn('"tconstruct:casting_table_tipping" =>', lookup)
        manifest = (ROOT / 'horizon-mixin.toml').read_text()
        self.assertIn('id = "source-tip-clearing"', manifest)
        self.assertIn('id = "source-tip-clearing-tests"', manifest)


if __name__ == '__main__':
    unittest.main()
