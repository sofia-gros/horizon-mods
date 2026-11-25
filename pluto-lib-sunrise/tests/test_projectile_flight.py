"""Projectile spawn/flight boundary checks, not flight runtime proof.

The game's entity-tick telling does not cover projectiles (pinned
`native_entity_tick_pre`, API 146: mobs, items, minecarts, boats), so the
spawn and flight tellings ride PlutoLib's own reaches into the game's
projectile list: a head on `add_projectile`, the flight pass delivered at
the head of `tick_fangs` (the call the creature tick makes immediately
after `tick_projectiles`, whose own body refuses a return seam), a
game-side pass in the creatures module, and a sink in the entry point the
API crate installs. These checks pin that wiring so it cannot silently fall
away: the access module, the two reaches (with the weave rule for
require), the pure events and their fields, and the transport's
hand-offs between them.
"""
from pathlib import Path
import tomllib
import unittest

ROOT = Path(__file__).resolve().parents[1]
BUNDLE = ROOT / 'mod'
API = ROOT / 'mod' / 'api' / 'src'


class ProjectileFlightTests(unittest.TestCase):
    def manifest(self):
        return tomllib.loads((BUNDLE / 'sunrise-mod.toml').read_text(encoding='utf-8'))

    def injection(self, wanted):
        return next(row for row in self.manifest()['injection'] if row['id'] == wanted)

    def module(self, wanted):
        return next(row for row in self.manifest()['module'] if row['id'] == wanted)

    def source(self, name):
        return (BUNDLE / name).read_text(encoding='utf-8')

    def test_the_access_module_is_attached_to_the_entry_point(self):
        access = self.module('pluto-projectile-flight-access')
        self.assertEqual(access['file'], 'crates/horizon-client/src/entry.rs')
        self.assertEqual(access['name'], 'pluto_projectile_flight_access')
        self.assertEqual(access['source'], 'pluto-projectile-flight-access.rs')
        self.assertEqual(access['visibility'], 'pub')
        source = self.source('pluto-projectile-flight-access.rs')
        self.assertIn('pub fn set_sink', source)
        self.assertIn('pub fn hears_spawn', source)
        self.assertIn('pub fn hears_flight', source)

    def test_the_game_pass_lives_in_the_creatures_module(self):
        game = self.module('pluto-projectile-flight-game')
        self.assertEqual(game['file'], 'crates/horizon-client/src/game/creatures.rs')
        self.assertEqual(game['name'], 'pluto_projectile_flight_game')
        self.assertEqual(game['source'], 'pluto-projectile-flight-game.rs')
        self.assertEqual(game['visibility'], 'pub(crate)')
        source = self.source('pluto-projectile-flight-game.rs')
        self.assertIn('pub(crate) fn on_spawn', source)
        self.assertIn('pub(crate) fn after_tick', source)
        self.assertIn('use crate::entry::pluto_projectile_flight_access as access;', source)
        self.assertIn('access::spawn', source)
        self.assertIn('access::flight', source)

    def test_the_spawn_reach_is_a_head_on_add_projectile(self):
        reach = self.injection('pluto-projectile-spawn-head')
        self.assertEqual(reach['target'], "sunrise_client::game::creatures::Game<'_>::add_projectile")
        self.assertEqual(reach['kind'], 'HEAD')
        self.assertEqual(reach['helper'], 'pluto-projectile-spawn-head.rs')
        self.assertIn('on_spawn', self.source('pluto-projectile-spawn-head.rs'))

    def test_the_flight_reach_rides_the_tick_fangs_head_without_require(self):
        reach = self.injection('pluto-projectile-flight-return')
        # A RETURN on tick_projectiles is not weaveable (its body uses ?,
        # E_UNSUPPORTED_CONTROL_FLOW), so the flight pass rides the head of
        # tick_fangs, which the creature tick calls immediately after
        # tick_projectiles; on the error path the caller aborts with ?, so
        # the head is skipped exactly as a return would be.
        self.assertEqual(reach['target'], "sunrise_client::game::fangs::Game<'_>::tick_fangs")
        self.assertEqual(reach['kind'], 'HEAD')
        # This head carries bindings, so require would be E_SELECTOR
        # (require applies only to site injections, and to binding-free heads).
        self.assertNotIn('require', reach)
        self.assertEqual(reach['helper'], 'pluto-projectile-flight-return.rs')
        self.assertIn('after_tick', self.source('pluto-projectile-flight-return.rs'))

    def test_the_pure_events_carry_the_spawn_and_flight_facts(self):
        pure = (API / 'pure' / 'projectile_events_v1.rs').read_text(encoding='utf-8')
        for field in ('pub struct ProjectileSpawned', 'pub struct ProjectileFlight', 'pub enum FlightAction',
                      'pub struct ArrowFacts', 'owner_position', 'release_from_ground', 'pickup_stack'):
            self.assertIn(field, pure)
        self.assertIn('pub const SPAWN', pure)
        self.assertIn('pub const FLIGHT', pure)
        self.assertIn('fn spawn_heard', pure)
        self.assertIn('fn flight_heard', pure)

    def test_the_release_event_is_pure_and_the_transport_answers_it(self):
        pure = (API / 'pure' / 'item_api_v1.rs').read_text(encoding='utf-8')
        self.assertIn('pub struct ReleaseUsing', pure)
        self.assertIn('pub const RELEASE_USING', pure)
        self.assertIn('ticks_used', pure)
        hooks = (API / 'hooks.rs').read_text(encoding='utf-8')
        self.assertIn('fn release_using', hooks)
        self.assertIn('item_api_v1::released', hooks)
        self.assertIn('pluto_projectile_flight_access::set_sink', hooks)
        self.assertIn('projectile_events_v1::spawned', hooks)
        self.assertIn('projectile_events_v1::flight', hooks)


if __name__ == '__main__':
    unittest.main()
