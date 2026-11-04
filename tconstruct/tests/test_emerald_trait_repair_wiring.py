"""Guard the scoped Station material-repair Emerald trait connection."""
from pathlib import Path
import unittest
ROOT = Path(__file__).resolve().parents[1]
class EmeraldTraitRepairWiring(unittest.TestCase):
    def test_station_uses_resolved_trait_in_both_repair_paths(self):
        text = (ROOT / 'mixins/tconstruct/source-station-world.rs').read_text()
        self.assertIn('ordered_emerald_repair_amount(&tool, &traits.levels, &traits.order, per_item)', text)
        self.assertIn('ordered_emerald_repair_amount(original, &traits.levels, &traits.order, 1.0)', text)
        self.assertNotIn('traits.levels.get("tconstruct:emerald").copied().unwrap_or(0) > 0 { return None; }', text)
if __name__ == '__main__': unittest.main()
