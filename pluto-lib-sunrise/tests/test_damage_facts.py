"""Damage-event fact source boundary checks, not damage runtime proof.

A damage event that names a mob attacker or a mob victim needs facts the
host's damage hooks do not carry: what the mob behind a synchronous attack
holds and wears, the health a victim has as a hit arrives, and a queue the
game drains to drop an item at a living entity. These checks pin that
wiring so it cannot silently fall away: the access module in the entry
point, the woven reaches (two of them riding the injections that already
own their places), the pure event's new fields and the drop action, and
the transport's hand-offs between them.
"""
from pathlib import Path
import tomllib
import unittest

ROOT = Path(__file__).resolve().parents[1]
BUNDLE = ROOT / 'mod'
API = ROOT / 'mod' / 'api' / 'src'


class DamageFactsTests(unittest.TestCase):
    def manifest(self):
        return tomllib.loads((BUNDLE / 'sunrise-mod.toml').read_text(encoding='utf-8'))

    def injection(self, wanted):
        return next(row for row in self.manifest()['injection'] if row['id'] == wanted)

    def module(self, wanted):
        return next(row for row in self.manifest()['module'] if row['id'] == wanted)

    def source(self, name):
        return (BUNDLE / name).read_text(encoding='utf-8')

    def test_the_access_module_is_attached_to_the_entry_point(self):
        access = self.module('pluto-damage-facts-access')
        self.assertEqual(access['file'], 'crates/horizon-client/src/entry.rs')
        self.assertEqual(access['name'], 'pluto_damage_facts_access')
        self.assertEqual(access['source'], 'pluto-damage-facts-access.rs')
        self.assertEqual(access['visibility'], 'pub')
        source = self.source('pluto-damage-facts-access.rs')
        self.assertIn('pub fn set_sink', source)
        self.assertIn('pub fn push_drop', source)
        self.assertIn('pub fn take_drops', source)

    def test_the_attacker_snapshot_rides_the_existing_knockback_scope_injection(self):
        ids = [row['id'] for row in self.manifest()['injection']]
        self.assertNotIn('pluto-damage-attacker', ids)
        reach = self.injection('pluto-player-hurt')
        self.assertEqual(reach['target'], "sunrise_client::game::native_hooks::Game<'_>::knockback_scope")
        self.assertEqual(reach['kind'], 'HEAD')
        self.assertEqual(reach['helper'], 'pluto-player-hurt.rs')
        source = self.source('pluto-player-hurt.rs')
        self.assertIn('pluto_damage_facts_access::AttackerFacts', source)
        self.assertIn('main_hand_stack()', source)

    def test_the_victim_reaches_report_health_before_the_hit(self):
        reach = self.injection('pluto-damage-victim')
        self.assertEqual(reach['target'], 'sunrise_client::game::native_hooks::hurt_server')
        self.assertEqual(reach['kind'], 'HEAD')
        self.assertNotIn('require', reach)  # hurt_server is a free-function hook target: the weave accepts require only on site injections (E_SELECTOR)
        self.assertEqual(reach['helper'], 'pluto-damage-victim.rs')
        self.assertIn('Hurting::Mob', self.source('pluto-damage-victim.rs'))
        dragon = self.injection('pluto-damage-dragon')
        self.assertEqual(dragon['target'], 'sunrise_client::game::dragon::Dragon::hurt_health')
        self.assertEqual(dragon['kind'], 'HEAD')
        self.assertNotIn('require', dragon)  # Dragon::hurt_health is not a site injection: the weave accepts require only on site injections (E_SELECTOR, build-sp weave plan 2026-11-23)
        self.assertEqual(dragon['helper'], 'pluto-damage-dragon.rs')
        self.assertIn('dragon.health', self.source('pluto-damage-dragon.rs'))

    def test_the_drop_drain_rides_the_existing_player_tick_injection(self):
        ids = [row['id'] for row in self.manifest()['injection']]
        self.assertNotIn('pluto-damage-drop-apply', ids)
        reach = self.injection('pluto-mob-effect-apply-tick')
        self.assertEqual(reach['target'], "sunrise_client::game::lan_attributes::Game<'_>::update_player_attributes")
        self.assertEqual(reach['kind'], 'HEAD')
        self.assertEqual(reach['helper'], 'pluto-mob-effect-apply-tick.rs')
        source = self.source('pluto-mob-effect-apply-tick.rs')
        self.assertIn('pluto_damage_facts_access::take_drops()', source)
        self.assertIn('spawn_item_own', source)

    def test_the_pure_event_carries_the_facts_and_the_drop_action(self):
        pure = (API / 'pure' / 'entity_events_v1.rs').read_text(encoding='utf-8')
        for field in ('attacker_kind', 'attacker_is_player', 'attacker_main_hand', 'attacker_chest',
                      'attacker_attack_damage', 'target_health', 'target_attack_damage',
                      'attacker_launcher_hand', 'attacker_launcher_stack'):
            self.assertIn(field, pure)
        self.assertIn('DropItem { target: ActionTarget, stack: StackData }', pure)
        self.assertIn('fn note_attacker_facts', pure)
        self.assertIn('fn note_victim_facts', pure)
        self.assertIn('fn note_damage_parties', pure)
        self.assertIn('fn take_drop_actions', pure)
        self.assertIn('fn take_late_actions', pure)

    def test_the_launcher_telling_names_the_hand_that_shot(self):
        # No new injection: the telling rides the knockback scope's opening (the
        # player's own hit, entity id 0) and the ammunition event's weapon note.
        ids = [row['id'] for row in self.manifest()['injection']]
        self.assertNotIn('pluto-damage-launcher', ids)
        reach = self.injection('pluto-player-hurt')
        self.assertEqual(reach['target'], "sunrise_client::game::native_hooks::Game<'_>::knockback_scope")
        self.assertEqual(reach['kind'], 'HEAD')
        self.assertEqual(reach['helper'], 'pluto-player-hurt.rs')
        helper = self.source('pluto-player-hurt.rs')
        self.assertIn('game.held()', helper)
        self.assertIn('game.offhand', helper)
        self.assertIn('"minecraft:player"', helper)
        access = self.source('pluto-damage-facts-access.rs')
        self.assertIn('pub off_hand', access)
        projectile = (API / 'pure' / 'projectile_events_v1.rs').read_text(encoding='utf-8')
        self.assertIn('fn launcher_for', projectile)
        self.assertIn('PLAYER_WEAPON', projectile)

    def test_the_transport_installs_the_sink_and_drains_the_queues(self):
        hooks = (API / 'hooks.rs').read_text(encoding='utf-8')
        self.assertIn('pluto_damage_facts_access::set_sink', hooks)
        self.assertIn('fn living_incoming_damage_v3', hooks)
        self.assertIn('note_damage_parties', hooks)
        self.assertIn('take_late_actions', hooks)
        self.assertIn('take_drop_actions', hooks)
        self.assertIn('pluto_damage_facts_access::push_drop', hooks)


if __name__ == '__main__':
    unittest.main()
