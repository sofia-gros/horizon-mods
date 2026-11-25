"""Hanging variant source boundary checks, not frame/stand runtime proof.

Fancy item frames and armor stands live in the host's vanilla lists, so
their variant behaviour is woven: a transport provider the API crate
installs, a game module that runs the mechanics, and heads/returns on the
host's placement, interaction, tick, drop, break, dispense and command
paths. These checks pin that wiring so it cannot silently fall away.
"""
from pathlib import Path
import tomllib
import unittest

ROOT = Path(__file__).resolve().parents[1]
BUNDLE = ROOT / 'mod'


class HangingTests(unittest.TestCase):
    def manifest(self):
        return tomllib.loads((BUNDLE / 'horizon-mixin.toml').read_text(encoding='utf-8'))

    def injection(self, wanted):
        return next(row for row in self.manifest()['injection'] if row['id'] == wanted)

    def module(self, wanted):
        return next(row for row in self.manifest()['module'] if row['id'] == wanted)

    def test_the_transport_and_game_modules_are_attached(self):
        access = self.module('pluto-hanging-access')
        # client_lib.rs (the shared crate root), not entry.rs: the
        # loader compiles in horizon-client-lib, whose root has
        # no entry module; entity_nbt.rs itself is unattachable
        # (glob re-exports, E_MODULE_CONFLICT).
        self.assertEqual(access['file'], 'crates/horizon-client/src/client_lib.rs')
        self.assertEqual(access['name'], 'pluto_hanging_access')
        self.assertEqual(access['source'], 'pluto-hanging-access.rs')
        self.assertEqual(access['visibility'], 'pub')
        game = self.module('pluto-hanging-game')
        self.assertEqual(game['file'], 'crates/horizon-client/src/game.rs')
        self.assertEqual(game['name'], 'pluto_hanging_game')
        self.assertEqual(game['source'], 'pluto-hanging-game.rs')
        self.assertEqual(game['visibility'], 'pub(crate)')

    def test_frame_placement_and_interaction_are_cancellable_heads(self):
        place = self.injection('pluto-hanging-frame-place')
        self.assertEqual(place['target'], "horizon_client::game::frames::Game<'_>::place_item_frame")
        self.assertEqual(place['kind'], 'HEAD')
        self.assertTrue(place['cancellable'])
        self.assertEqual(place['helper'], 'pluto-hanging-frame-place.rs')
        interact = self.injection('pluto-hanging-frame-interact')
        self.assertEqual(interact['target'], "horizon_client::game::frames::Game<'_>::interact_frame")
        self.assertEqual(interact['kind'], 'HEAD')
        self.assertTrue(interact['cancellable'])
        self.assertEqual(interact['helper'], 'pluto-hanging-frame-interact.rs')

    def test_the_loader_applies_the_variant_rotation(self):
        load = self.injection('pluto-hanging-frame-from-nbt')
        self.assertEqual(load['target'], 'horizon_client::entity_nbt::hanging::item_frame_from_nbt')
        self.assertEqual(load['kind'], 'HEAD')
        self.assertTrue(load['cancellable'])
        source = (BUNDLE / 'pluto-hanging-frame-from-nbt.rs').read_text(encoding='utf-8')
        self.assertIn('frame_for_variant(variant)', source)
        self.assertIn('frame_rotation(&facts, saved)', source)

    def test_tick_analog_explosion_and_drops_are_wired(self):
        tick = self.injection('pluto-hanging-frame-tick')
        self.assertEqual(tick['target'], "horizon_client::game::frames::Game<'_>::tick_item_frames")
        self.assertEqual(tick['kind'], 'HEAD')
        analog = self.injection('pluto-hanging-frame-analog')
        self.assertEqual(analog['target'], "horizon_client::game::<GameLevel<'_> as LevelAccess>::item_frame_analog_output")
        self.assertEqual(analog['kind'], 'HEAD')
        self.assertTrue(analog['cancellable'])
        blast = self.injection('pluto-hanging-frame-explosion-head')
        self.assertEqual(blast['target'], "horizon_client::game::frames::Game<'_>::explosion_breaks_frames")
        self.assertEqual(blast['kind'], 'HEAD')
        # The restore cannot be a RETURN on the frames pass (that body is
        # not weaveable), so it rides the head of the decorations pass the
        # explosion sequence makes after it, under the entry's original id.
        restore = self.injection('pluto-hanging-frame-explosion-return')
        self.assertEqual(restore['target'], "horizon_client::game::explosion::Game<'_>::explosion_breaks_decorations")
        self.assertEqual(restore['kind'], 'HEAD')
        drops = self.injection('pluto-hanging-frame-drops')
        self.assertEqual(drops['target'], "horizon_client::game::frames::Game<'_>::drop_frame_items")
        self.assertTrue(drops['cancellable'])

    def test_item_removal_and_command_set_reset_the_rotation(self):
        hit = self.injection('pluto-hanging-frame-hit')
        self.assertEqual(hit['target'], "horizon_client::game::frames::Game<'_>::hit_frame")
        self.assertEqual(hit['kind'], 'RETURN')
        set_item = self.injection('pluto-hanging-frame-set-item')
        self.assertEqual(set_item['target'], "horizon_client::game::frames::Game<'_>::set_frame_item")
        self.assertEqual(set_item['kind'], 'HEAD')

    def test_stand_paths_are_wired(self):
        place = self.injection('pluto-hanging-stand-place')
        self.assertEqual(place['target'], "horizon_client::game::armor_stands::Game<'_>::place_armor_stand")
        self.assertTrue(place['cancellable'])
        brk = self.injection('pluto-hanging-stand-break')
        self.assertEqual(brk['target'], "horizon_client::game::armor_stands::Game<'_>::break_armor_stand_by_hit")
        self.assertTrue(brk['cancellable'])
        head = self.injection('pluto-hanging-stand-tick-head')
        self.assertEqual(head['target'], "horizon_client::game::armor_stands::Game<'_>::tick_armor_stands")
        self.assertEqual(head['kind'], 'HEAD')
        # The after-pass cannot be a RETURN on the stand tick (that body
        # cannot be woven), so it rides the head of the call the pinned
        # tick sequence makes immediately after the stand tick.
        after = self.injection('pluto-hanging-stand-tick-after')
        self.assertEqual(after['target'], "horizon_client::game::cushions::Game<'_>::tick_cushions")
        self.assertEqual(after['kind'], 'HEAD')
        dispense = self.injection('pluto-hanging-stand-dispense')
        self.assertEqual(dispense['target'], "horizon_client::game::dispenser::Game<'_>::dispense_item")
        self.assertTrue(dispense['cancellable'])
        command = self.injection('pluto-hanging-stand-command')
        self.assertEqual(command['target'], "horizon_client::game::armor_stands::Game<'_>::command_damage_stand")
        self.assertTrue(command['cancellable'])

    def test_the_game_module_runs_the_variant_mechanics(self):
        source = (BUNDLE / 'pluto-hanging-game.rs').read_text(encoding='utf-8')
        # The variant rides in the kept raw NBT, as the original saves it.
        self.assertIn('"Variant"', source)
        self.assertIn('"RotationTimer"', source)
        self.assertIn('"LeftHanded"', source)
        # Placement, timers, click-through, explosion stash, drops, onPlace,
        # the break drop and the fire refusals all live in the game module.
        for needle in ('place_fancy_frame', 'interact_fancy_frame', 'tick_fancy_frames',
                       'click_through', 'stash_immune_frames', 'restore_immune_frames',
                       'fancy_frame_drops', 'place_fancy_stand', 'break_fancy_stand',
                       'tick_fancy_stands', 'dispense_fancy_stand', 'fancy_stand_refuses'):
            self.assertIn(needle, source)

    def test_hooks_install_the_provider(self):
        source = (ROOT / 'mod' / 'api' / 'src' / 'hooks.rs').read_text(encoding='utf-8')
        self.assertIn('pluto_hanging_access::set_provider', source)
        self.assertIn('native_hanging_frame_for_item', source)
        self.assertIn('HangingRegistry::frame_for_variant', source)


if __name__ == '__main__':
    unittest.main()
