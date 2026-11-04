"""Guard the bounded Cheese Block finish hook and declared food components."""
import json, unittest
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
class CheeseRemainder(unittest.TestCase):
 def test_declared_food_and_finish_wiring(self):
  d=json.loads((ROOT/'content/data/tconstruct/horizon/block/cheese_block.json').read_text())
  self.assertNotIn('minecraft:food',d.get('components',{})); self.assertNotIn('minecraft:consumable',d.get('components',{}))
  m=(ROOT/'mixins/tconstruct/horizon-mixin.toml').read_text()
  self.assertIn('id = "source-cheese-finish"',m); self.assertIn('native_living_entity_use_item_finish',m)
  h=(ROOT/'mixins/tconstruct/source-cheese-finish.rs').read_text()
  self.assertIn('entity.is_some() || game.remote.is_some()',h)
  self.assertIn('ItemStack::new("tconstruct:cheese_ingot", 3)',h)
  self.assertIn('crate::game::inventory_add',h); self.assertIn('*result = game.held().cloned()',h); self.assertIn('game.give(remainder)',h);self.assertLess(h.index('held.count -= 1'),h.index('game.trigger_item'))
if __name__=='__main__':unittest.main()
