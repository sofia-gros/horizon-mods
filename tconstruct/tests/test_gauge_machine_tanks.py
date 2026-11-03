"""Guard the routing from machine gauge interaction to nested tank decoding."""
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1] / "mixins" / "tconstruct"

class GaugeMachineTanks(unittest.TestCase):
    def test_machine_reads_use_machine_kind_and_nested_decoder(self):
        gauge = (ROOT / "source-gauge.rs").read_text()
        runtime = (ROOT / "source-machine-runtime.rs").read_text()
        fuel = (ROOT / "source-smeltery-fuel.rs").read_text()
        self.assertIn('"tconstruct:seared_casting_tank" => Some("tconstruct:casting_tank")', runtime)
        self.assertIn('"tconstruct:seared_fluid_cannon" => Some("tconstruct:fluid_cannon")', runtime)
        self.assertIn('gauge_tank(block, &store::read(world, pos, kind)?)', gauge)
        self.assertIn('"tconstruct:seared_casting_tank" | "tconstruct:seared_fluid_cannon" => saved.get("tank")?', fuel)
        self.assertIn('Tank::from_nbt(tank, capacity)', fuel)

    def test_placement_and_interaction_share_read_path(self):
        gauge = (ROOT / "source-gauge.rs").read_text()
        self.assertIn('tank(level.world, level.facts, level.registry, name, at, facing).is_some()', gauge)
        self.assertIn('tank(&game.world, game.res.facts(), game.pack.assets.registry, block, at, facing)', gauge)

if __name__ == '__main__':
    unittest.main()
