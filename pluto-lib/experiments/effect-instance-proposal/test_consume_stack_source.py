"""Source checks only: no staging, Rust compiler, native fixture or execution."""
import hashlib
import os
from pathlib import Path
import unittest

import prepare_consume_stack as p


class ConsumedStackSource(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.official = Path(os.environ["PLUTO_OFFICIAL_SOURCE"])
        cls.old = p.read_sources(cls.official)
        cls.new = p.transform(cls.old)

    def test_exact_artifact(self):
        artifact = Path(__file__).with_name("UNAPPLIED-consumed-stack-cure-boundary.patch")
        self.assertEqual(artifact.read_text(), p.patch(self.old))

    def test_only_two_files_change(self):
        self.assertEqual({k for k in self.old if self.old[k] != self.new[k]}, {p.GAME, p.CONSUME})

    def test_game_inverse_is_byte_exact(self):
        restored = p.replace_once(self.new[p.GAME], p.NEW_SIGNATURE, p.OLD_SIGNATURE)
        restored = p.replace_once(restored, p.NEW_CLEAR, p.OLD_CLEAR)
        restored = p.replace_once(restored, p.HELPER + p.ANCHOR, p.ANCHOR)
        self.assertEqual(restored, self.old[p.GAME])

    def test_consume_inverse_is_byte_exact(self):
        self.assertEqual(p.replace_once(self.new[p.CONSUME], p.NEW_CALL, p.OLD_CALL), self.old[p.CONSUME])
        # The original emission/listener/effect/event/decrement/remainder order stays unchanged.
        source = self.new[p.CONSUME]
        anchors = ["self.emit_consume(consumable, stack, 16)", 'self.award_stat("used"',
                   'self.trigger_item("minecraft:consume_item"', "if let Some((nutrition, saturation))",
                   "for (id, duration) in &data.stew_effects", "for effect in &consumable.effects",
                   "self.apply_consume_effect(effect, stack)", "self.post_entity_event(event, Who::Player)",
                   "self.consume_held()", "if let Some(remainder) = &info.use_remainder"]
        positions = [source.index(a) for a in anchors]
        self.assertEqual(positions, sorted(positions))

    def test_no_new_policy_or_owner_resolution(self):
        self.assertEqual(p.HELPER[p.HELPER.index("    fn "):],
            "    fn clear_effects_from_consumed_item(&mut self, _consumed_stack: &ItemStack) {\n"
            "        self.potions.effects.clear();\n        self.apply_effect_status();\n    }\n")
        self.assertEqual(p.HELPER.count("_consumed_stack"), 1)
        self.assertNotIn("register", p.HELPER)
        self.assertNotIn("lan_acting", p.HELPER)

    def test_existing_callers_and_current_slot_distinction(self):
        game = self.old[p.GAME]
        item_use = self.old["crates/horizon-client/src/game/item_use.rs"]
        self.assertEqual(game.count("self.finish_consuming(info, consumable, &stack)"), 1)
        self.assertEqual(item_use.count("self.finish_consuming(info, consumable, stack)"), 1)
        clear_to_tail = self.new[p.GAME].split(p.NEW_CLEAR, 1)[1].split(p.ANCHOR, 1)[0]
        for unchanged in ["ConsumeEffect::RemoveEffects(to_remove)",
                          "ConsumeEffect::TeleportRandomly(teleport) => self.teleport_randomly(teleport)",
                          "self.native_consume_effect(kind, data)"]:
            self.assertIn(unchanged, clear_to_tail)
        self.assertIn("let cell = self.held_mut();", game)

    def test_existing_kernel_is_not_new_game_authority(self):
        source = self.old["crates/horizon-world/src/consumable.rs"]
        self.assertIn("pub trait ConsumptionWorld", source)
        self.assertIn("fn apply_effect(", source)
        self.assertIn("world.apply_effect(effect, level, stack.clone())?", source)
        self.assertNotIn("ConsumptionWorld", self.old[p.CONSUME])
        self.assertNotIn("ConsumptionWorld", self.old[p.GAME])
        effect = self.old["crates/horizon-world/src/consume_effect.rs"]
        self.assertIn("Self::ClearStatus => world.remove_all_effects()", effect)

    def test_native_instance_and_persistence_unchanged(self):
        source = self.old["crates/horizon-client-render/src/effects.rs"]
        self.assertIn("pub fn saved(&self) -> Vec<(String, i32, i32, bool, bool)>", source)
        for relative, expected in p.HASHES.items():
            self.assertEqual(hashlib.sha256((self.official / relative).read_bytes()).hexdigest(), expected)


if __name__ == "__main__":
    unittest.main()
