"""Check reload and level-entry wiring for the host's broad-tag snapshot."""
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1] / 'mixins/tconstruct'

class BroadWireLifecycle(unittest.TestCase):
    def test_reload_invalidates_parked_levels_before_active_capture(self):
        for name in ('recipe-command.rs', 'recipe-pack.rs'):
            source = (ROOT / name).read_text()
            self.assertLess(source.index('invalidate_parked_item_tags(game)'), source.index('capture(game)'))
        reload = (ROOT / 'recipe-reload.rs').read_text().split('fn invalidate_parked_item_tags', 1)[1]
        for dimension in ('Overworld', 'Nether', 'End'):
            self.assertIn(f'crate::game::Dim::{dimension}', reload)
        self.assertIn('parked_world_mut(dim)', reload)
        self.assertIn('__horizon_mixins_tconstruct_resolved_item_tags_mut() = None', reload)

    def test_level_tick_recaptures_only_missing_snapshot(self):
        manifest = (ROOT / 'horizon-mixin.toml').read_text().split('id = "recipe-level-enter-refresh"', 1)[1]
        self.assertIn("horizon_client::game::Game<'_>::update_world", manifest)
        self.assertIn('kind = "HEAD"', manifest.split('[[', 1)[0])
        source = (ROOT / 'recipe-level-enter.rs').read_text()
        self.assertIn('__horizon_mixins_tconstruct_resolved_item_tags().is_none()', source)
        self.assertIn('game.res.rest.get().is_some()', source)
        self.assertNotIn('game.fns.loaded', source)
        self.assertIn('game.ensure_datapacks()', source)

if __name__ == '__main__':
    unittest.main()
