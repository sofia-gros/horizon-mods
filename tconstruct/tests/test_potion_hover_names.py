"""Guards for potion names in local machine hover tooltips."""
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1] / 'mixins' / 'tconstruct'


class PotionHoverNames(unittest.TestCase):
    def test_known_local_tags_use_source_checked_potion_name(self):
        render = (ROOT / 'source-melter-render.rs').read_text()
        self.assertIn('fluid == "tconstruct:potion" && metadata_complete', render)
        self.assertIn('potion_fluid_name(tag)', render)
        self.assertIn('game.pack.names.display_get(&key)', render)
        self.assertIn('fluid_name(game, fluid)', render)

    def test_only_complete_local_machine_metadata_is_used(self):
        tooltip = (ROOT / 'station-result-tooltip.rs').read_text()
        self.assertIn('fluid_name_tagged(game, fluid, tank.tag.as_ref(), game.remote.is_none())', tooltip)
        self.assertIn('fluid_name_tagged(game, fluid, state.tank.tag.as_ref(), game.remote.is_none())', tooltip)
        self.assertIn('fluid_name_tagged(game, &fluid.id, fluid.tag.as_ref(), game.remote.is_none())', tooltip)
        # FuelView omits tags, so it must retain the generic fluid name.
        self.assertIn('fluid_name(game, fluid)', tooltip)


if __name__ == '__main__':
    unittest.main()
