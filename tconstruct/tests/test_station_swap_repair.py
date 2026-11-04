from pathlib import Path
import unittest
ROOT=Path(__file__).resolve().parents[1]
class StationSwapRepair(unittest.TestCase):
    def test_actual_station_rebuilds_before_hook_and_consumption(self):
        s=(ROOT/'mixins/tconstruct/source-station-world.rs').read_text()
        s=s[s.index('fn swap_plan_with_capacity'):s.index('/// Rebuild using')]
        self.assertLess(s.index('part_swap_repair_base'),s.index('if !changed'))
        self.assertLess(s.index('rebuild_material_swap'),s.index('ordered_known_repair_amount'))
        self.assertIn('levels(world, &output)',s)
        self.assertIn('ordered_known_repair_amount(&output, &traits.levels, &traits.order, initial)',s)
        self.assertLess(s.index('ordered_known_repair_amount'),s.index('finish_station_swap_repair'))
        self.assertIn('finish_station_swap_repair(&mut output, was_broken, repair)?',s)
        self.assertIn('vec![(0, 1), (slot, 1)]',s)
        self.assertNotIn('tool.damage.saturating_sub',s)
if __name__=='__main__': unittest.main()
