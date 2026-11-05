"""Native signal and commit-route wiring; behavior runs in the host fixture."""
from pathlib import Path
import unittest
ROOT = Path(__file__).resolve().parents[1]
class MelterComparator(unittest.TestCase):
    def test_signal_hook_and_all_runtime_commits(self):
        mixins = ROOT / 'mixins/tconstruct'
        hook = (mixins / 'source-casting-comparator.rs').read_text()
        present = (mixins / 'source-native-hook-bodies.rs').read_text().split('pub(super) fn tconstruct_geode_random_tick')[0]
        self.assertIn('"tconstruct:seared_melter"', present)
        self.assertIn('Melter::from_nbt(&tag)).map_or(0, |state| state.comparator())', hook)
        runtime = (mixins / 'source-machine-runtime.rs').read_text()
        tick = runtime.split('pub(super) fn tick_melter(',1)[1].split('pub(super) fn casting_at(',1)[0]
        self.assertIn('if !write_machine_updates(game, &updates)', tick)
        faucet = (mixins / 'source-faucet-tank.rs').read_text()
        self.assertNotIn('store::write_many(', faucet)
        use = (mixins / 'source-machine-use.rs').read_text().split('"tconstruct:seared_melter" =>',1)[1].split('"tconstruct:copper_gauge" =>',1)[0]
        self.assertLess(use.index('store::write'), use.index('game.container_changed(pos)'))
    def test_ci_executes_actual_commit_fixture(self):
        ci = (ROOT.parent / '.github/workflows/mixins.yml').read_text()
        self.assertIn('python3 tconstruct/tests/run_faucet_tank_casting.py', ci)
        runner = (ROOT/'tests/run_faucet_tank_casting.py').read_text()
        self.assertIn('source.index("pub(super) fn write_machine_updates(")', runner)
if __name__ == '__main__': unittest.main()
