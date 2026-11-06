"""Source and complete-plan guard tests; not live packet execution."""
import copy
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
    # Synthetic acceptance context only; production metadata stays pending.
    def meta(self):
        meta=guard.metadata()
        meta["candidate_observation"]={"horizon_revision":meta["horizon_revision"],"plan_sha256":"a"*64,"index_result_sha256":"b"*64,"evidence_commit":"c"*40}
        meta["caller_fingerprints"][meta["callers"][1]]="d"*64
        return meta
    def plan(self):
        meta=self.meta()
        return {"inputs":meta["source_sha256"],"injections":[{
            "mod_id":"pluto_native_creative","injection_id":meta["injection_id"],
            "target":meta["target"],**meta["location"],"kind":"RETURN","cancellable":False,
            "function_fingerprint":meta["function_fingerprint"],"bindings":meta["bindings"],
            "helper_source":(NATIVE/meta["helper"]).read_text()}],"modules":[{
            "mod_id":"pluto_native_creative","module_id":meta["internal_updates"]["module_id"],
            "file":meta["internal_updates"]["file"],"name":meta["internal_updates"]["name"],
            "source":(NATIVE/meta["internal_updates"]["source"]).read_text()}]}
    def test_exact_guard_and_native_phase_source(self):
        guard.check(self.plan(),self.meta())
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
            with self.assertRaises(ValueError):guard.check(plan,self.meta())
    def test_old_caller_cancellation_and_interiors_are_rejected(self):
        meta=self.meta()
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
            with self.assertRaises(ValueError):guard.check(plan,self.meta())
    def test_alias_target_spellings_cannot_hide_the_same_resolved_function(self):
        plan=self.plan();other=copy.deepcopy(plan["injections"][0])
        other.update(mod_id="other", target="different::spelling", kind="HEAD", cancellable=True)
        plan["injections"].append(other)
        with self.assertRaises(ValueError):guard.check(plan,self.meta())
        for fingerprint in self.meta()["caller_fingerprints"].values():
            plan=self.plan();plan["injections"].append({"target":"other::spelling", "kind":"HEAD", "cancellable":True,"function_fingerprint":fingerprint})
            with self.assertRaises(ValueError):guard.check(plan,self.meta())

    def test_private_update_module_missing_duplicate_and_foreign_placement_reject(self):
        for change in [lambda p:p.update(modules=[]),
                lambda p:p["modules"].append(copy.deepcopy(p["modules"][0])),
                lambda p:p["modules"][0].update(mod_id="other"),
                lambda p:p["modules"][0].update(module_id="unexpected-id"),
                lambda p:p["modules"][0].update(file="crates/horizon-client/src/game/books.rs"),
                lambda p:p["modules"][0].update(name="other_name"),
                lambda p:p["modules"][0].update(visibility="pub"),
                lambda p:p["modules"][0].update(source="// changed")]:
            plan=self.plan();change(plan)
            with self.assertRaises(ValueError):guard.check(plan,self.meta())
        plan=self.plan();plan["modules"][0]["module_id"]="pluto_native_creative-"+plan["modules"][0]["module_id"]
        guard.check(plan,self.meta())
        plan=self.plan();plan["modules"][0]["visibility"]="pub(crate)"
        guard.check(plan,self.meta())

    def test_published_pending_observation_and_missing_caller_always_reject(self):
        pending=guard.metadata()
        pending["candidate_observation"]=None
        pending["caller_fingerprints"][pending["callers"][1]]=None
        with self.assertRaisesRegex(ValueError,"actual plan/Index"):
            guard.check(self.plan(),pending)
        for value in [None,"", "unknown", "a"*63]:
            meta=self.meta();meta["caller_fingerprints"][meta["callers"][1]]=value
            with self.assertRaisesRegex(ValueError,"unobserved"):
                guard.check(self.plan(),meta)

    def test_observation_must_name_exact_revision_and_evidence(self):
        for key,value in [("horizon_revision","wrong"),("plan_sha256",None),("index_result_sha256","not-observed"),("evidence_commit","x"*40)]:
            meta=self.meta();meta["candidate_observation"][key]=value
            with self.assertRaisesRegex(ValueError,"actual plan/Index"):
                guard.check(self.plan(),meta)
        meta=self.meta();meta["caller_fingerprints"].pop(meta["callers"][1])
        with self.assertRaisesRegex(ValueError,"unobserved"):
            guard.check(self.plan(),meta)

    def test_private_update_alias_placement_cannot_hide_a_second_module(self):
        plan=self.plan();extra=copy.deepcopy(plan["modules"][0]);extra.update(mod_id="other",module_id="other")
        plan["modules"].append(extra)
        with self.assertRaises(ValueError):guard.check(plan,self.meta())

    def test_private_update_source_order_and_no_public_game_callback(self):
        meta=self.meta();source=(NATIVE/meta["internal_updates"]["source"]).read_text().split("#[cfg(test)]")[0]
        helper=(NATIVE/meta["helper"]).read_text()
        self.assertLess(helper.index("::snapshot(packet)"),helper.index("::observe(packet)"))
        self.assertLess(helper.index("::observe(packet)"),helper.index("snapshot.dispatch(game)"))
        self.assertLess(source.index("let context = (self.handler.context)(game)"),source.index("game.book.as_mut()"))
        self.assertIn("pub(super) type Context = for<'g> fn(&Game<'g>) -> u64;",source)
        self.assertIn("pub(super) type Update = for<'v, 'p> fn(&mut RetainedView<'v>, u64, Payload<'p>);",source)
        self.assertNotIn("fn(&mut Game",source)
        self.assertEqual(source.count("(self.handler.update)("),1)

    def test_pristine_source_hashes_fail_closed(self):
        with tempfile.TemporaryDirectory() as directory:
            workspace=Path(directory)
            with self.assertRaises(OSError):guard.check_source(workspace,self.meta())
    def test_packaged_module_and_return_only_no_state(self):
        manifest=tomllib.loads((NATIVE/"horizon-mixin.toml").read_text())
        hooks=[h for h in manifest["injection"] if h["id"].startswith("client-payload")]
        self.assertEqual(len(hooks),1);self.assertEqual(hooks[0]["kind"],"RETURN")
        self.assertEqual(hooks[0]["bindings"],self.meta()["bindings"])
        self.assertFalse(any(s["id"].startswith("client-payload") for s in manifest.get("state",[])))
        spec=importlib.util.spec_from_file_location("payload_package",ROOT/"tools/package_native.py")
        package=importlib.util.module_from_spec(spec);spec.loader.exec_module(package)
        _,files=package._bundle(json.loads((ROOT/"horizon-native.json").read_text()))
        for name in ["client-payload.rs","client-payload-observed.rs","client-payload-updates.rs","inventory-api/src/client_payload.rs"]:
            self.assertEqual(files[f"pluto_native_creative/{name}"],(NATIVE/name).read_bytes())

if __name__=="__main__":unittest.main()
