"""Source ownership checks; official planner/type checks belong to the host batch."""
from pathlib import Path
import tomllib
import unittest

ROOT = Path(__file__).resolve().parents[2]
NATIVE = ROOT / 'pluto-lib/mixins/pluto_native_creative'


class MotionQueryBoundary(unittest.TestCase):
    def test_one_private_game_module_without_runtime_hooks(self):
        manifest = tomllib.loads((NATIVE / 'horizon-mixin.toml').read_text())
        self.assertEqual([row for row in manifest['module'] if row['id'] == 'motion-query'], [{
            'id': 'motion-query', 'file': 'crates/horizon-client/src/game.rs',
            'name': 'pluto_motion_query', 'source': 'motion-query.rs',
        }])
        for section in ('injection', 'state', 'crate'):
            self.assertFalse(any(row['id'] == 'motion-query' for row in manifest.get(section, [])))

    def test_pluto_owns_complete_active_game_motion_context(self):
        source = (NATIVE / 'motion-query.rs').read_text()
        for field in ('entities: &[]', 'game.player.as_mut()', 'scene: &mut game.world',
                      'assets: &game.pack.assets', 'collision_binding: game.collision',
                      'resources: game.motion', 'health: &game.health_resources'):
            self.assertIn(field, source)
        self.assertIn('return Ok(None)', source)
        self.assertIn('.climbable()\n    .map(Some)', source)
        self.assertNotIn('tconstruct', source)
        self.assertNotIn('unwrap_or', source)
        self.assertNotIn('Default::default', source)

    def test_consumer_retains_eligibility_and_conservative_error_policy(self):
        source = (ROOT / 'tconstruct/mixins/tconstruct/source-projectile-airborne.rs').read_text()
        for policy in ('game.riding.is_some()', 'else { return false; }',
                       'player.ground.on_ground', 'pluto_fluid_facts::player(player).touching_water',
                       'pluto_motion_query::climbable(game).ok().flatten().unwrap_or(true)',
                       'airborne_conditions(false, false, false, climbable)'):
            self.assertIn(policy, source)
        self.assertNotIn('MotionWorld', source)
        self.assertNotIn('health_resources', source)
        self.assertNotIn('collision_binding', source)


if __name__ == '__main__':
    unittest.main()
