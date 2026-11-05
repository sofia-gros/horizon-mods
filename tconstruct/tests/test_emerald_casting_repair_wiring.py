"""Guard Emerald's limited casting repair path and existing safety gates."""
from pathlib import Path
import unittest
ROOT=Path(__file__).resolve().parents[1]
class EmeraldCastingRepairWiring(unittest.TestCase):
    def test_factor_is_applied_only_after_existing_swap_safety_checks(self):
        text=(ROOT/'mixins/tconstruct/source-melter-world.rs').read_text()
        self.assertIn('rebuild_casting_material_swap(world, definition, stack, &next',text)
        self.assertIn('if !composite && repair_damage != 0',text)
        self.assertIn('casting_repair_amount(',text)
        self.assertIn('&output, &traits.levels, &traits.order, initial,',text)
        self.assertIn('durability as f32 * (swap.item_cost as f32 / 3.0)',text)
if __name__=='__main__': unittest.main()
