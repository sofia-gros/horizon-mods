"""Pinned source-order observations, not a new borrow/native execution proof."""
import hashlib
import json
import os
from pathlib import Path
import subprocess
import unittest


class ContactBorrowPhases(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.root = Path(os.environ["PLUTO_OFFICIAL_SOURCE"]).resolve()
        cls.record = json.loads((Path(__file__).parents[2] / "docs/contact-borrow-phase-map.json").read_text())
        cls.sources = {}
        actual = subprocess.check_output(["git", "-C", str(cls.root), "rev-parse", "HEAD"], text=True).strip()
        if actual != cls.record["official_revision"]:
            raise ValueError("Wrong official revision")
        for path, expected in cls.record["official_sha256"].items():
            data = (cls.root / path).read_bytes()
            if hashlib.sha256(data).hexdigest() != expected:
                raise ValueError("Pinned official source drift: " + path)
            cls.sources[path] = data.decode()
        cls.lan = cls.sources["crates/horizon-client/src/game/lan_actions.rs"]
        start = cls.lan.index("    fn guest_server_tick(")
        cls.turn = cls.lan[start:cls.lan.index("        let Some((events, marked)) = events", start)]
        cls.motion = cls.sources["crates/horizon-client/src/motion.rs"]

    def order(self, source, anchors):
        positions = [source.index(a) for a in anchors]
        self.assertEqual(positions, sorted(positions))

    def test_native_motion_owned_tail_precedes_game_handoff(self):
        self.order(self.turn, ["let mut w = MotionWorld", "w.fluid_base_tick()",
            "w.server_block_damage(start)", "w.update_soul_speed_block()",
            "w.tick_environment_damage()", "w.apply_fire_contact()", "w.server_hazards_tick(in_wall)",
            "w.server_player_pose_tick()", "std::mem::take(&mut w.player.events)",
            "game.cauldron_extinguish", "game.rain_extinguish_player()", "game.wither_rose_contact()"])

    def test_deferred_rose_is_not_a_game_call_inside_contact(self):
        start = self.motion.index("    fn check_inside_blocks(")
        part = self.motion[start:]
        self.order(part, ["visited.insert(p.pack())", "crate::block_contact::Event::Inside(",
                          "self.player.events.push(MotionEvent::InsideWitherRose)"])
        self.assertNotIn("game.wither_rose_contact", part)
        creatures = self.sources["crates/horizon-client/src/game/creatures.rs"]
        rose = creatures.split("pub(super) fn wither_rose_contact", 1)[1].split("pub(super) fn hurt_player_by", 1)[0]
        self.order(rose, ["self.remote.is_some() || self.ui.difficulty == 0", 'self.give_player_effect("minecraft:wither", 40, 0)'])

    def test_earlier_values_cross_existing_reborrow(self):
        main_turn = self.turn.split("let events = self.as_guest(host, conn, |game| {", 1)[1]
        self.order(main_turn, ["let rules = game.world.health_rules", "let tick_movement =", "let start =",
            "game.tick_mob_effects()", "game.handle_shoulder_entities()", "player.peaceful_regeneration(rules)",
            "player.carry_client_movement(tick_movement)", ".server_travel()", "game.touch_pufferfish()",
            "game.record_combat(&events)", "game.wear_armor(&events)"])
        tail = main_turn.split("game.handle_shoulder_entities();", 1)[1]
        self.assertIn("let Some(player) = game.player.as_mut() else", tail)
        self.assertNotIn("ready_server_epoch", tail)
        self.assertNotIn("native_entity_ref", tail)

    def test_inner_error_and_outer_ignore_remain_distinct(self):
        self.assertIn("let _ = w.server_block_damage(start);", self.turn)
        start = self.motion.index("    pub fn server_block_damage(")
        body = self.motion[start:self.motion.index("    fn effects_from_blocks(", start)]
        self.order(body, ["self.effects_from_blocks(true)?", "self.player.ticks_frozen =", "self.freeze_damage()?"])

    def test_static_contact_still_has_no_game_borrow(self):
        source = self.sources["crates/horizon-client/src/block_contact.rs"]
        self.assertIn("OnceLock<fn(&str, Event<'_>, &mut f32)>", source)
        self.assertNotIn("Game<", source)
        self.assertIn("LEVEL.set(self.0)", source)

    def test_vanilla_content_callback_branch_is_narrow_not_global_proof(self):
        source = self.sources["crates/horizon-client/src/game/potions.rs"]
        start = source.index("pub(super) fn add_player_effect(")
        body = source[start:source.index("fn mod_effect_tick(", start)]
        self.order(body, ["self.potions.effects.add(effect)", "self.apply_effect_status()", 'id == "absorption"',
                          "self.mod_effect_added(id, None, amp, new)"])
        self.assertIn("self.remote.is_none() && brewing::is_mod_effect(effect)", body)
        core = self.sources["crates/horizon-client-core/src/potions.rs"]
        self.assertIn('!id.starts_with("minecraft:") && id.bytes().any(|b| b == b\':\')', core)

    def test_lower_motion_has_registry_but_not_behaviours_or_effect_owner(self):
        motion = self.motion.split("pub struct MotionWorld<", 1)[1].split("enum TickInput", 1)[0]
        for name in ["player", "scene", "assets", "resources", "health"]:
            self.assertIn("pub " + name + ":", motion)
        self.assertNotIn("PlayerEffects", motion)
        self.assertNotIn("Behaviours", motion)
        assets = self.sources["crates/horizon-client-render/src/assets.rs"].split("pub struct Assets {", 1)[1].split("pub struct RenderAssets", 1)[0]
        self.assertIn("pub registry: &'static BlockRegistry", assets)
        self.assertNotIn("Behaviours", assets)

    def test_sources_still_exact_after_reading(self):
        for path, expected in self.record["official_sha256"].items():
            self.assertEqual(hashlib.sha256((self.root / path).read_bytes()).hexdigest(), expected)


if __name__ == "__main__":
    unittest.main()
