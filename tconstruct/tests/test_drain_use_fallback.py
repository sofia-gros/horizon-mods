"""Guard the drain interaction hook's pass-through contract."""
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]


class DrainUseFallbackTest(unittest.TestCase):
    def test_unhandled_drain_use_reaches_normal_item_placement(self):
        source = (ROOT / 'mixins/tconstruct/source-machine-use.rs').read_text()
        branch = source.split('"tconstruct:seared_drain" | "tconstruct:scorched_drain" => {', 1)[1].split('\n        },', 1)[0]
        self.assertIn('if !crate::game::tconstruct_source_machine_runtime::interact_drain(game, pos) { return Continue(()); }', branch)
        self.assertTrue(branch.rstrip().endswith('true'))
        hook = (ROOT / 'mixins/tconstruct/horizon-mixin.toml').read_text().split('id = "source-machine-use"', 1)[1].split('[[', 1)[0]
        self.assertIn('kind = "HEAD"', hook)
        self.assertIn('cancellable = true', hook)

if __name__ == '__main__':
    unittest.main()
