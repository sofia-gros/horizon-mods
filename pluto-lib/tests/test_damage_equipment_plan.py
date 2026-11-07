import copy
import importlib.util
from pathlib import Path
import unittest

SPEC = importlib.util.spec_from_file_location("damage_equipment_plan", Path(__file__).resolve().parents[1] / "tools/check_damage_equipment_plan.py")
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)

META = [{"target": "game::ordinary", "injection_id": "equipment-ordinary-replacement", "function_fingerprint": "official"},
        {"target": "game::explosion", "injection_id": "equipment-explosion-replacement", "function_fingerprint": "official2"}]


def plan():
    return {"injections": [dict(mod_id="pluto_native_creative", injection_id=m["injection_id"], target=m["target"],
        kind="HEAD", priority=-2147483648, cancellable=True, function_fingerprint=m["function_fingerprint"]) for m in META]}


class DamageEquipmentPlanTests(unittest.TestCase):
    def test_native_heads_and_returns_can_compose_once(self):
        candidate = plan()
        for kind in ("HEAD", "RETURN"):
            candidate["injections"].append(dict(mod_id="other", injection_id=kind, target="game::ordinary", kind=kind, priority=0))
        MODULE.check_order(candidate, META)

    def test_packaged_prefixed_ids_are_accepted(self):
        candidate = plan()
        for hook in candidate["injections"]:
            hook["injection_id"] = "pluto_native_creative-" + hook["injection_id"]
        MODULE.check_order(candidate, META)

    def test_every_interior_kind_is_rejected(self):
        for kind in ("OVERWRITE", "BEFORE_CALL", "AFTER_CALL", "AFTER_ASSIGN", "MODIFY_ARG", "MODIFY_ARGS", "MODIFY_VARIABLE", "MODIFY_EXPRESSION_VALUE", "MODIFY_CONSTANT", "WRAP_OPERATION"):
            with self.subTest(kind=kind):
                candidate = plan()
                candidate["injections"].append(dict(mod_id="other", injection_id=kind, target="game::ordinary", kind=kind, priority=100))
                with self.assertRaisesRegex(ValueError, "bypassed"):
                    MODULE.check_order(candidate, META)

    def test_competing_lowest_head_is_rejected(self):
        candidate = plan()
        candidate["injections"].append(dict(mod_id="other", injection_id="late", target="game::explosion", kind="HEAD", priority=-2147483648))
        with self.assertRaisesRegex(ValueError, "follow"):
            MODULE.check_order(candidate, META)

    def test_missing_and_duplicate_replacements_are_rejected(self):
        candidate = plan()
        candidate["injections"].pop()
        with self.assertRaisesRegex(ValueError, "expected one"):
            MODULE.check_order(candidate, META)
        candidate = plan()
        candidate["injections"].append(copy.deepcopy(candidate["injections"][0]))
        with self.assertRaisesRegex(ValueError, "expected one"):
            MODULE.check_order(candidate, META)

    def test_source_fingerprint_drift_is_rejected(self):
        candidate = plan()
        candidate["injections"][0]["function_fingerprint"] = "new source"
        with self.assertRaisesRegex(ValueError, "fingerprint"):
            MODULE.check_order(candidate, META)

    def test_wrong_replacement_phase_is_rejected(self):
        for key, value in (("kind", "RETURN"), ("priority", 0), ("cancellable", False)):
            with self.subTest(key=key):
                candidate = plan()
                candidate["injections"][0][key] = value
                with self.assertRaisesRegex(ValueError, "lowest cancellable"):
                    MODULE.check_order(candidate, META)
