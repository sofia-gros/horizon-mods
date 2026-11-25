"""Lightweight armor observer source/composition contracts, not host execution."""
import importlib.util
import json
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location("armor_damage_guard", ROOT / "tools/check_damage_equipment_plan.py")
GUARD = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(GUARD)
META = json.loads((ROOT / "tools/armor-wear-source.json").read_text())


def plan():
    return {"injections": [dict(
        mod_id="pluto_native_creative", injection_id=META["injection_id"],
        target=META["target"], kind="HEAD", priority=-2147483648,
        cancellable=True, function_fingerprint=META["function_fingerprint"],
    )]}


class ArmorWearObservationTests(unittest.TestCase):
    def test_third_caller_is_in_required_preflight(self):
        self.assertIn(META, GUARD.entries())
        GUARD.check_order(plan(), [META])

    def test_interior_hook_or_competing_replacement_fails_closed(self):
        for kind, priority in [("MODIFY_VARIABLE", 1), ("BEFORE_CALL", 0), ("HEAD", -2147483648)]:
            candidate = plan()
            candidate["injections"].append(dict(mod_id="other", injection_id="other", target=META["target"], kind=kind, priority=priority))
            with self.assertRaises(ValueError):
                GUARD.check_order(candidate, [META])

    def test_source_fingerprint_drift_fails_closed(self):
        candidate = plan()
        candidate["injections"][0]["function_fingerprint"] = "changed"
        with self.assertRaisesRegex(ValueError, "fingerprint"):
            GUARD.check_order(candidate, [META])

    def test_body_is_not_changed_and_finish_follows_native_call(self):
        self.assertEqual(META["changes"], [])
        self.assertIn("pluto_armor_wear::begin", META["header"])
        footer = META["footer"]
        self.assertLess(footer.index("original(game, source, damage)"), footer.index("observation.finish(game)"))
        self.assertLess(footer.index("observation.finish(game)"), footer.index("ControlFlow::Break"))
        self.assertNotIn("hurt_armor(", footer)  # No recursive target call.

    def test_earlier_heads_and_later_returns_have_explicit_phase(self):
        candidate = plan()
        for kind in ["HEAD", "RETURN"]:
            candidate["injections"].append(dict(mod_id="other", injection_id=kind, target=META["target"], kind=kind, priority=0))
        GUARD.check_order(candidate, [META])

    def test_native_nonpositive_gate_precedes_any_observation(self):
        header = META["header"]
        self.assertLess(header.index("if damage <= 0.0"), header.index("pluto_armor_wear::begin"))
        self.assertIn("return std::ops::ControlFlow::Continue(())", header)
        self.assertNotIn("is_finite", header)
        self.assertEqual(META["changes"], [])

    def test_bridge_has_no_ambient_state_and_rechecks_authority(self):
        source = (ROOT / "mixins/pluto_native_creative/armor-wear-observation.rs").read_text()
        self.assertIn("Scope::capture(game).as_ref() != Some(&self.scope)", source)
        self.assertIn("game.remote.is_some()", source)
        self.assertIn("equipment_allowed(player)", source)
        for field in ["entity", "dimension", "world_root", "guest", "guest_uuid", "guest_entity", "creative", "spectator"]:
            self.assertIn(f"{field}:", source)
        for forbidden in ["thread_local!", "static ", "&mut Game", "hurt_armor(", "stack.damage ="]:
            self.assertNotIn(forbidden, source)


if __name__ == "__main__":
    unittest.main()
