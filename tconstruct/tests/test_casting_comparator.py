from pathlib import Path
import unittest
ROOT=Path(__file__).resolve().parents[1]
class CastingComparator(unittest.TestCase):
    def test_tables_and_basins_expose_signals(self):
        p=ROOT/'mixins/tconstruct'
        hook=(p/'source-casting-comparator.rs').read_text()
        present=(p/'source-native-hook-bodies.rs').read_text().split('pub(super) fn tconstruct_geode_random_tick')[0]
        for metal in ('seared','scorched'):
            for block in ('table','basin'):
                self.assertIn(f'tconstruct:{metal}_{block}',hook)
                self.assertIn(f'tconstruct:{metal}_{block}',present)
    def test_progress_interaction_and_hopper_notifications(self):
        p=ROOT/'mixins/tconstruct'
        runtime=(p/'source-machine-runtime.rs').read_text()
        for start,end in [('pub(super) fn interact_casting(', 'fn smeltery_source('),('pub(super) fn tick_casting(',None)]:
            body=runtime.split(start,1)[1]
            if end:body=body.split(end,1)[0]
            self.assertNotIn('store::write(',body)
            self.assertIn('write_machine_updates(game,',body)
        hopper=(p/'source-chute-hopper.rs').read_text().split('let Some(patched)',1)[1]
        self.assertLess(hopper.index('tconstruct_source_melter_store::write'),hopper.index('game.container_changed(target)'))
if __name__=='__main__':unittest.main()
