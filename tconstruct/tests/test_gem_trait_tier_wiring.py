"""Guard both supported rebuild entry points for resolved gem trait tiers."""
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]

class GemTraitTierWiring(unittest.TestCase):
    def test_station_and_creative_pass_traits_and_harvest_membership(self):
        station = (ROOT / 'mixins/tconstruct/source-station-world.rs').read_text()
        creative = (ROOT / 'mixins/tconstruct/resource-tables.rs').read_text()
        self.assertIn('tank_traits.as_ref(), has_tag("tconstruct:modifiable/harvest")', station)
        self.assertIn('tank_traits, harvest,', station)
        self.assertIn('traits.as_ref().map(|(levels, _)| levels), harvest.iter().any(|id| id == item)', creative)

if __name__ == '__main__':
    unittest.main()
