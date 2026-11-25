import copy
import hashlib
import tempfile
from unittest.mock import patch
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


class NativeEquipmentPlanTests(unittest.TestCase):
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
        self.plan = {'schema': 6, 'inputs': dict(sources), 'injections': [],
                     'states': [], 'modules': [{'mod_id': 'pluto_lib', 'file': 'unrelated.rs'}]}

    def test_native_sources_bind_plan_and_workspace(self):
        MODULE.check_order(self.plan, META)
        MODULE.verify_source(self.plan, self.root, META)
        for name in MODULE.NATIVE_SOURCES:
            source = self.root / name
            source.write_bytes(b'changed direct patch')
            with self.assertRaisesRegex(ValueError, 'workspace source differs'):
                MODULE.verify_source(self.plan, self.root, META)
            source.write_bytes(b'native fixture')
            expected = self.plan['inputs'].pop(name)
            with self.assertRaisesRegex(ValueError, 'plan source differs'):
                MODULE.verify_source(self.plan, self.root, META)
            self.plan['inputs'][name] = expected
            source.unlink()
            with self.assertRaisesRegex(ValueError, 'not a regular file'):
                MODULE.verify_source(self.plan, self.root, META)
            source.write_bytes(b'native fixture')

    def test_native_boundaries_reject_attachments_and_legacy_composition(self):
        for section in ('injections', 'modules', 'states'):
            candidates = ([{'mod_id': 'foreign', 'target': target}
                           for target in MODULE.NATIVE_TARGETS | {m['target'] for m in META}
                           | {'alias::Game::hurt_player_from', 'alias::Game::hurt_armor'}]
                          + [{'mod_id': 'foreign', 'file': name} for name in MODULE.UNATTACHED_FILES]
                          + [{'mod_id': 'pluto_native_creative', 'file': 'unrelated.rs'}])
            for entry in candidates:
                with self.subTest(section=section, entry=entry):
                    self.plan[section].append(entry)
                    with self.assertRaises(ValueError):
                        MODULE.check_order(self.plan, META)
                    with self.assertRaises(ValueError):
                        MODULE.verify_source(self.plan, self.root, META)
                    self.plan[section].pop()
        self.plan['schema'] = 7
        with self.assertRaisesRegex(ValueError, 'schema'):
            MODULE.check_order(self.plan, META)

    def test_unrelated_creature_hook_is_not_an_equipment_replacement(self):
        self.plan['injections'].append({
            'mod_id': 'pluto_lib', 'file': 'crates/horizon-client/src/game/creatures.rs',
            'target': "sunrise_client::game::creatures::Game<'_>::spawn_mob_reason"})
        MODULE.check_order(self.plan, META)
        MODULE.verify_source(self.plan, self.root, META)


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
