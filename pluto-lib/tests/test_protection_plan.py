import importlib.util
from pathlib import Path
import unittest

SPEC = importlib.util.spec_from_file_location("protection_plan", Path(__file__).resolve().parents[1] / "tools/check_protection_plan.py")
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


def plan():
    return {"injections": [
        {"mod_id": "pluto_native_creative", "injection_id": "protection-player-scope", "target": MODULE.HIT, "kind": "HEAD", "priority": 2147483647},
        {"mod_id": "pluto_native_creative", "injection_id": "protection-project", "target": MODULE.STATUS, "kind": "RETURN", "priority": -10000},
        {"mod_id": "pluto_native_creative", "injection_id": "protection-kernel", "target": MODULE.KERNEL, "kind": "HEAD", "priority": 100},
    ]}


class ProtectionPlanTests(unittest.TestCase):
    def test_isolated_and_ordinary_consumer(self):
        candidate = plan()
        MODULE.check(candidate)
        candidate["injections"].append({"mod_id": "example", "injection_id": "lower-head", "target": MODULE.HIT, "kind": "HEAD", "priority": 100})
        MODULE.check(candidate)

    def test_competing_head_is_rejected(self):
        candidate = plan()
        candidate["injections"].append({"mod_id": "aaa", "injection_id": "competing-head", "target": MODULE.HIT, "kind": "HEAD", "priority": 2147483647})
        with self.assertRaisesRegex(ValueError, "twice"):
            MODULE.check(candidate)

    def test_late_numeric_return_is_rejected(self):
        candidate = plan()
        candidate["injections"].append({"mod_id": "example", "injection_id": "late-return", "target": MODULE.STATUS, "kind": "RETURN", "priority": -10001})
        with self.assertRaisesRegex(ValueError, "precede"):
            MODULE.check(candidate)

    def test_hit_returns_and_missing_consumer_are_rejected(self):
        candidate = plan()
        candidate["injections"].append({"mod_id": "example", "injection_id": "hit-return", "target": MODULE.HIT, "kind": "RETURN", "priority": 100})
        with self.assertRaisesRegex(ValueError, "different wrapper"):
            MODULE.check(candidate)
        candidate = plan()
        candidate["injections"].pop()
        with self.assertRaisesRegex(ValueError, "missing"):
            MODULE.check(candidate)

    def test_scalar_return_cannot_observe_private_marker(self):
        candidate = plan()
        candidate["injections"].append({"mod_id": "example", "injection_id": "scalar-return", "target": MODULE.KERNEL, "kind": "RETURN", "priority": 100})
        with self.assertRaisesRegex(ValueError, "marker"):
            MODULE.check(candidate)
