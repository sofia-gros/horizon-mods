"""Guard snapshot capture and machine-specific ore-rate wiring."""
from pathlib import Path
import unittest

M = Path(__file__).resolve().parents[1] / 'mixins/tconstruct'

class OreRateLifecycle(unittest.TestCase):
    def test_capture_clear_and_machine_paths(self):
        reload = (M / 'recipe-reload.rs').read_text()
        self.assertIn('tconstruct_source_config::load_all(&root)', reload)
        self.assertIn('__horizon_mixins_tconstruct_source_ore_rates_mut() = Some(ore_rates)', reload)
        self.assertIn('__horizon_mixins_tconstruct_source_ore_rates_mut() = None',
                      (M / 'part-selection-chunk-clear.rs').read_text())
        runtime = (M / 'source-machine-runtime.rs').read_text()
        self.assertIn('melting_stack_for(&game.world, stack, no_crafting_remainder(game, stack), "smeltery")', runtime)
        self.assertIn('melting_stack(&game.world, stack, no_crafting_remainder(game, stack))', runtime)
        world = (M / 'source-melter-world.rs').read_text()
        self.assertIn('ore_rates(world, "foundry"), ore_rates(world, "foundry_byproduct")', world)

if __name__ == '__main__': unittest.main()
