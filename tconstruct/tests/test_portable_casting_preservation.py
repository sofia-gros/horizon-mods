from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]


class PortableCastingPreservation(unittest.TestCase):
    def test_recipe_outputs_use_the_shared_preserving_planner(self):
        world = (ROOT / 'mixins/tconstruct/source-melter-world.rs').read_text()
        ordinary = world.split('pub(super) fn casting_stack(', 1)[1].split('if !basin {', 1)[0]
        self.assertIn('portable_tank(stack, capacity)?', ordinary)
        self.assertIn('fill_portable(stack, tank, fluid, None, recipe.fluid.amount)?', ordinary)
        self.assertNotIn('tank.to_nbt()', ordinary)
        self.assertIn('fill_portable_tagged(stack, tank, fluid, tag, recipe.fluid.amount)?', world)
        state = (ROOT / 'mixins/tconstruct/source-melter-state.rs').read_text()
        planner = state.split('pub(super) fn fill_portable(', 1)[1].split('#[derive(Clone, Debug)]', 1)[0]
        self.assertIn('portable_tank(stack, tank.capacity).as_ref() != Some(&tank)', planner)
        self.assertIn('tank.to_nbt_preserving(raw)?', planner)
        self.assertIn('fill_portable(stack, tank, fluid, Some(tag), amount)', planner)

    def test_existing_fixture_requires_preservation_cases(self):
        runner = (ROOT / 'tests/run_drain_container.py').read_text()
        for case in ('portable_recipe_fill_preserves_all_twelve_tanks',
                     'portable_recipe_fill_rejects_ambiguous_and_opaque_inputs',
                     'portable_recipe_fill_requires_exact_current_tank',
                     'portable_recipe_fill_keeps_extensions_through_casting_completion'):
            self.assertIn(case, runner)


if __name__ == '__main__':
    unittest.main()
