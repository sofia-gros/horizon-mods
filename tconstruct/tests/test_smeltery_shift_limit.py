"""Wiring checks for one-item quick moves; not an in-game menu test."""
from pathlib import Path
import unittest
ROOT = Path(__file__).resolve().parents[1]
class SmelteryShiftLimit(unittest.TestCase):
 def test_local_and_guest_use_one_item_limit_on_insertion(self):
  for name in ('source-smeltery-click.rs','source-smeltery-guest-guard.rs'):
   text=(ROOT/'mixins/tconstruct'/name).read_text()
   self.assertIn('source_give_inventory(&mut state.slots, 0, stack, 1, false)',text)
   self.assertNotIn('source_give_inventory(&mut state.slots, 0, stack, max, stackable)',text)
   self.assertIn('source_quick_move_player(&mut inventory, stack, max, stackable)',text)
if __name__=='__main__': unittest.main()
