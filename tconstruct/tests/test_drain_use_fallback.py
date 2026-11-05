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

    def test_supported_failed_transfer_is_handled_without_changing_save_failure(self):
        source = (ROOT / 'mixins/tconstruct/source-machine-runtime.rs').read_text()
        route = source.split('pub(super) fn interact_drain(',1)[1].split('pub(super) fn ensure(',1)[0]
        self.assertIn('accepts_with_color(&held',route)
        self.assertIn('return container;',route)
        self.assertIn('state.to_nbt()) { return false; }',route)
        self.assertLess(route.index('store::write'),route.index('game.give_or_drop'))
        ci = (ROOT.parent / '.github/workflows/mixins.yml').read_text()
        self.assertIn('python3 tconstruct/tests/run_drain_container.py',ci)

if __name__ == '__main__':
    unittest.main()
