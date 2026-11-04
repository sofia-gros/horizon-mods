"""Both harvest boundaries must retain the broken-tool guard."""
from pathlib import Path
import unittest
ROOT = Path(__file__).resolve().parents[1]
class BrokenHarvestFlags(unittest.TestCase):
    def test_both_active_harvest_hooks(self):
        manifest = (ROOT / 'mixins/tconstruct/horizon-mixin.toml').read_text()
        for name, kind, helper in [('broken-harvest', 'HEAD', 'broken-harvest.rs'), ('source-material-harvest-tier', 'RETURN', 'source-harvest-return.rs')]:
            rule = manifest.split(f'id = "{name}"', 1)[1].split('[[', 1)[0]
            self.assertIn('target = "horizon_items::items::Items::correct_for_drops"', rule)
            self.assertIn(f'kind = "{kind}"', rule)
            self.assertIn(f'helper = "{helper}"', rule)
