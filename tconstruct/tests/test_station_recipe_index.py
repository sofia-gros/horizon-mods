"""Check that the station recipe index follows the world snapshot lifecycle."""
from pathlib import Path
import unittest

MIXINS = Path(__file__).resolve().parents[1] / 'mixins/tconstruct'


class StationRecipeIndexLifecycle(unittest.TestCase):
    def test_capture_rebuilds_and_world_clear_discards_index(self):
        capture = (MIXINS / 'recipe-reload.rs').read_text()
        self.assertIn('building_recipe_index(&recipes)', capture)
        self.assertLess(capture.index('building_recipe_index(&recipes)'),
                        capture.index('__horizon_mixins_tconstruct_recipe_snapshot_mut() = Some(recipes)'))
        clear = (MIXINS / 'part-selection-chunk-clear.rs').read_text()
        self.assertIn('__horizon_mixins_tconstruct_station_building_recipes_mut() = None', clear)
        self.assertIn('__horizon_mixins_tconstruct_recipe_snapshot_mut() = None', clear)
        self.assertIn('tconstruct_recipe_reload::capture(game)', (MIXINS / 'recipe-command.rs').read_text())
        self.assertIn('tconstruct_recipe_reload::capture(game)', (MIXINS / 'recipe-ensure.rs').read_text())


if __name__ == '__main__':
    unittest.main()
