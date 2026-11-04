"""Both mining-speed boundaries must retain the broken-tool guard."""
from pathlib import Path
import unittest
ROOT = Path(__file__).resolve().parents[1]
class BrokenMiningSpeedFlags(unittest.TestCase):
    def test_both_active_mining_speed_hooks(self):
        manifest = (ROOT / 'mixins/tconstruct/horizon-mixin.toml').read_text()
        for name, kind, helper in [('broken-speed', 'HEAD', 'broken-speed.rs'), ('source-material-mining-speed', 'RETURN', 'source-mining-return.rs')]:
            rule = manifest.split(f'id = "{name}"', 1)[1].split('[[', 1)[0]
            self.assertIn('target = "horizon_items::items::Items::mining_speed"', rule)
            self.assertIn(f'kind = "{kind}"', rule)
            self.assertIn(f'helper = "{helper}"', rule)
if __name__ == '__main__':
    unittest.main()
