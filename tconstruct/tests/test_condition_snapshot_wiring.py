"""Keep the Forge gate on the shared snapshot path, ahead of consumers."""
import pathlib
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[1]


class ConditionSnapshotWiring(unittest.TestCase):
    def test_reload_uses_resolved_condition_bytes(self):
        reload_source = (ROOT / 'mixins/tconstruct/recipe-reload.rs').read_text()
        block = reload_source.split('recipes.retain(|_, bytes| {', 1)[1].split('    });', 1)[0]
        self.assertIn('conditional_recipe(bytes, registries, config, items, Some(&mod_presence))', block)
        self.assertIn('resolved_recipe_conditions(bytes, registries, config, items, Some(&mod_presence))', block)
        self.assertIn('unconditional_recipe(bytes)', block)
        self.assertLess(block.index('unconditional_recipe(bytes)'), block.rindex('let Some((registries, config, items))'))
        self.assertEqual(block.count('*bytes = '), 3)

    def test_ci_runs_production_resolver_tests(self):
        workflow = (ROOT.parent / '.github/workflows/mixins.yml').read_text()
        self.assertIn('cargo test --locked --manifest-path tconstruct/tests/condition-snapshot/Cargo.toml', workflow)


if __name__ == '__main__':
    unittest.main()
