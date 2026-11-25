"""Source and complete-plan guard tests; not live packet execution."""
import copy
import hashlib
from unittest.mock import patch
import importlib.util
import json
from pathlib import Path
import tempfile
import tomllib
import unittest
ROOT=Path(__file__).resolve().parents[1]
NATIVE=ROOT/"mixins/pluto_native_creative"
spec=importlib.util.spec_from_file_location("client_payload_guard",ROOT/"tools/check_client_payload_plan.py")
guard=importlib.util.module_from_spec(spec);spec.loader.exec_module(guard)

class ClientPayloadTests(unittest.TestCase):
    def plan(self):
        meta=guard.metadata()
        return {"inputs":meta["source_sha256"],"injections":[{
            "mod_id":"pluto_native_creative","injection_id":meta["injection_id"],
            "target":meta["target"],**meta["location"],"kind":"RETURN","cancellable":False,
            "function_fingerprint":meta["function_fingerprint"],"bindings":meta["bindings"],
            "helper_source":(NATIVE/meta["helper"]).read_text()}]}
    def test_exact_guard_and_native_phase_source(self):
        guard.check(self.plan(),guard.metadata())
        helper=(NATIVE/"client-payload-observed.rs").read_text()
        self.assertIn("execution_role.is_client()",helper)
        self.assertNotIn("native_client_packet(",helper)
        self.assertNotIn("send(",helper)
        projection=(NATIVE/"client-payload.rs").read_text().split("#[cfg(test)]")[0]
        self.assertIn("if !api::enabled()",projection)
        self.assertNotIn(".clone()",projection)
        self.assertNotIn("to_vec()",projection)
    def test_native_observer_foreign_hooks_fail_closed(self):
        for kind in ["HEAD","RETURN","MODIFY_ARG","AFTER_CALL"]:
            plan=self.plan();extra=copy.deepcopy(plan["injections"][0]);extra.update(mod_id="other",kind=kind)
            plan["injections"].append(extra)
            with self.assertRaises(ValueError):guard.check(plan,guard.metadata())
    def test_old_caller_cancellation_and_interiors_are_rejected(self):
        meta=guard.metadata()
        for target in meta["callers"]:
            for kind,cancel in [("HEAD",True),("RETURN",True),("MODIFY_VARIABLE",False),("REDIRECT",False)]:
                plan=self.plan();plan["injections"].append({"target":target,"kind":kind,"cancellable":cancel})
                with self.assertRaises(ValueError):guard.check(plan,meta)
            for kind in ["HEAD","RETURN"]:
                plan=self.plan();plan["injections"].append({"target":target,"kind":kind,"cancellable":False})
                guard.check(plan,meta)
    def test_missing_duplicate_bindings_helper_and_source_drift_are_rejected(self):
        for change in [lambda p:p.update(injections=[]),
                lambda p:p["injections"].append(copy.deepcopy(p["injections"][0])),
                lambda p:p["injections"][0].update(bindings=[]),
                lambda p:p["injections"][0].update(helper_source="fn changed() {}"),
                lambda p:p["injections"][0].update(function_fingerprint="bad"),
                lambda p:p.update(inputs={})]:
            plan=self.plan();change(plan)
            with self.assertRaises(ValueError):guard.check(plan,guard.metadata())
    def test_alias_target_spellings_cannot_hide_the_same_resolved_function(self):
        plan=self.plan();other=copy.deepcopy(plan["injections"][0])
        other.update(mod_id="other", target="different::spelling", kind="HEAD", cancellable=True)
        plan["injections"].append(other)
        with self.assertRaises(ValueError):guard.check(plan,guard.metadata())
        for fingerprint in guard.metadata()["caller_fingerprints"].values():
            plan=self.plan();plan["injections"].append({"target":"other::spelling", "kind":"HEAD", "cancellable":True,"function_fingerprint":fingerprint})
            with self.assertRaises(ValueError):guard.check(plan,guard.metadata())

    def test_pristine_source_hashes_fail_closed(self):
        with tempfile.TemporaryDirectory() as directory:
            workspace=Path(directory)
            with self.assertRaises(OSError):guard.check_source(workspace,guard.metadata())
    def test_packaged_module_and_return_only_no_state(self):
        manifest=tomllib.loads((NATIVE/"sunrise-mod.toml").read_text())
        hooks=[h for h in manifest["injection"] if h["id"].startswith("client-payload")]
        self.assertEqual(len(hooks),1);self.assertEqual(hooks[0]["kind"],"RETURN")
        self.assertEqual(hooks[0]["bindings"],guard.metadata()["bindings"])
        self.assertFalse(any(s["id"].startswith("client-payload") for s in manifest.get("state",[])))
        spec=importlib.util.spec_from_file_location("payload_package",ROOT/"tools/package_native.py")
        package=importlib.util.module_from_spec(spec);spec.loader.exec_module(package)
        _,files=package._bundle(json.loads((ROOT/"horizon-native.json").read_text()))
        for name in ["client-payload.rs","client-payload-observed.rs","inventory-api/src/client_payload.rs"]:
            self.assertEqual(files[f"pluto_native_creative/{name}"],(NATIVE/name).read_bytes())

class NativePayloadTests(unittest.TestCase):
    def setUp(self):
        temp = tempfile.TemporaryDirectory()
        self.addCleanup(temp.cleanup)
        self.root = Path(temp.name)
        sources = {}
        for name in guard.NATIVE_SOURCES:
            source = self.root / name
            source.parent.mkdir(parents=True, exist_ok=True)
            source.write_bytes(b'native packet fixture')
            sources[name] = hashlib.sha256(source.read_bytes()).hexdigest()
        patcher = patch.object(guard, 'NATIVE_SOURCES', sources)
        patcher.start()
        self.addCleanup(patcher.stop)
        self.plan = {'schema': 6, 'inputs': dict(sources), 'injections': [],
                     'modules': [{'mod_id': 'pluto_lib', 'file': 'unrelated.rs'}], 'states': []}

    def test_current_sources_include_poll_loop_and_reject_drift(self):
        meta = guard.metadata(self.plan)
        guard.check(self.plan, meta)
        guard.check_source(self.root, meta)
        for name in guard.NATIVE_SOURCES:
            expected = self.plan['inputs'].pop(name)
            with self.assertRaisesRegex(ValueError, 'plan source differs'):
                guard.check(self.plan, meta)
            self.plan['inputs'][name] = expected
            source = self.root / name
            source.write_bytes(b'changed')
            with self.assertRaisesRegex(ValueError, 'source drifted'):
                guard.check_source(self.root, meta)
            source.write_bytes(b'native packet fixture')
        with self.assertRaisesRegex(ValueError, 'metadata'):
            guard.check(self.plan, guard.metadata())
        self.plan['schema'] = 7
        with self.assertRaisesRegex(ValueError, 'schema'):
            guard.check(self.plan, meta)

    def test_native_callers_and_mixed_bundle_remain_forbidden(self):
        meta = guard.metadata(self.plan)
        candidates = ([{'mod_id': 'foreign', 'target': 'alias::Game::' + name}
                       for name in guard.NATIVE_METHODS]
                      + [{'mod_id': 'foreign', 'file': name} for name in guard.NATIVE_SOURCES
                         if name.endswith(('remote_poll.rs', 'execution_role.rs'))]
                      + [{'mod_id': 'pluto_native_creative', 'file': 'unrelated.rs'}])
        for section in ('injections', 'modules', 'states'):
            for entry in candidates:
                with self.subTest(section=section, entry=entry):
                    self.plan[section].append(entry)
                    with self.assertRaises(ValueError):
                        guard.check(self.plan, meta)
                    self.plan[section].pop()
        self.plan['injections'].append({'mod_id': 'pluto_lib',
                                       'target': "sunrise_client::game::native_hooks::Game<'_>::native_screen_data",
                                       'file': 'crates/horizon-client/src/game/native_hooks.rs'})
        guard.check(self.plan, meta)


if __name__=="__main__":unittest.main()
