import importlib.util
from pathlib import Path
import unittest
import hashlib
import tempfile
from unittest.mock import patch

SPEC = importlib.util.spec_from_file_location("protection_plan", Path(__file__).resolve().parents[1] / "tools/check_protection_plan.py")
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


def plan():
    return {"injections": [
        {"mod_id": "pluto_native_creative", "injection_id": "protection-player-scope", "target": MODULE.HIT, "kind": "HEAD", "priority": 2147483647},
        {"mod_id": "pluto_native_creative", "injection_id": "protection-project", "target": MODULE.STATUS, "kind": "RETURN", "priority": -10000},
        {"mod_id": "pluto_native_creative", "injection_id": "protection-kernel", "target": MODULE.KERNEL, "kind": "HEAD", "priority": 100},
    ]}


class NativeProtectionTests(unittest.TestCase):
    def setUp(self):
        temp = tempfile.TemporaryDirectory()
        self.addCleanup(temp.cleanup)
        self.root = Path(temp.name)
        sources = {}
        for name in MODULE.NATIVE_SOURCES:
            source = self.root / name
            source.parent.mkdir(parents=True, exist_ok=True)
            source.write_bytes(b'native fixture')
            sources[name] = hashlib.sha256(source.read_bytes()).hexdigest()
        patcher = patch.object(MODULE, 'NATIVE_SOURCES', sources)
        patcher.start()
        self.addCleanup(patcher.stop)
        self.plan = {'schema': 6, 'inputs': dict(sources), 'injections': [], 'states': [],
                     'modules': [{'mod_id': 'pluto_lib', 'file': 'unrelated.rs'}]}

    def test_requires_matching_workspace_and_plan(self):
        MODULE.check(self.plan, self.root)
        self.plan['schema'] = 7
        with self.assertRaisesRegex(ValueError, 'schema'):
            MODULE.check(self.plan, self.root)
        self.plan['schema'] = 6
        with self.assertRaisesRegex(ValueError, 'requires the input workspace'):
            MODULE.check(self.plan)
        name = next(iter(MODULE.NATIVE_SOURCES))
        (self.root / name).write_bytes(b'changed')
        with self.assertRaisesRegex(ValueError, 'workspace source differs'):
            MODULE.check(self.plan, self.root)
        (self.root / name).write_bytes(b'native fixture')
        self.plan['inputs'][name] = 'changed'
        with self.assertRaisesRegex(ValueError, 'plan source differs'):
            MODULE.check(self.plan, self.root)
        self.plan['inputs'].pop(name)
        with self.assertRaisesRegex(ValueError, 'plan source differs'):
            MODULE.check(self.plan, self.root)

    def test_all_attachment_kinds_and_mixed_legacy_bundle_reject(self):
        for section in ('injections', 'modules', 'states'):
            for name in MODULE.NATIVE_SOURCES:
                with self.subTest(section=section, file=name):
                    self.plan[section].append({'mod_id': 'foreign', 'file': name})
                    with self.assertRaisesRegex(ValueError, 'must remain unattached'):
                        MODULE.check(self.plan, self.root)
                    self.plan[section].pop()
        for target in (MODULE.HIT, MODULE.STATUS, MODULE.KERNEL):
            self.plan['injections'].append({'mod_id': 'foreign', 'target': target})
            with self.assertRaisesRegex(ValueError, 'must remain unattached'):
                MODULE.check(self.plan, self.root)
            self.plan['injections'].pop()
        self.plan['modules'].append({'mod_id': 'pluto_native_creative', 'file': 'elsewhere.rs'})
        with self.assertRaisesRegex(ValueError, 'retired marker bundle'):
            MODULE.check(self.plan, self.root)

    def test_missing_native_source_and_empty_plan_reject(self):
        (self.root / next(iter(MODULE.NATIVE_SOURCES))).unlink()
        with self.assertRaisesRegex(ValueError, 'not a regular file'):
            MODULE.check(self.plan, self.root)
        with self.assertRaisesRegex(ValueError, 'exactly one'):
            MODULE.check({}, self.root)


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
