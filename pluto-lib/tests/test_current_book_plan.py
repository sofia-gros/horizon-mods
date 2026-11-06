"""Guard/source evidence only; these do not execute a native book turn."""
import copy
import importlib.util
import json
from pathlib import Path
import tomllib
import unittest
ROOT=Path(__file__).resolve().parents[1];NATIVE=ROOT/"mixins/pluto_native_creative"
spec=importlib.util.spec_from_file_location("current_book_guard",ROOT/"tools/check_current_book_plan.py")
guard=importlib.util.module_from_spec(spec);spec.loader.exec_module(guard)
class CurrentBookTests(unittest.TestCase):
    def plan(self):
        meta=guard.metadata()
        return {"inputs":meta["official_sources"],"injections":[],"modules":[{"mod_id":"pluto_native_creative","module_id":meta["module_id"],"file":meta["module_file"],"name":meta["module_name"],"source":(NATIVE/meta["source"]).read_text()}]}
    def test_exact_reviewed_module_without_hooks(self):guard.check(self.plan(),guard.metadata())
    def test_any_native_turn_hook_or_alias_is_rejected(self):
        meta=guard.metadata()
        for kind in ["HEAD","RETURN","AFTER_CALL","MODIFY_VARIABLE"]:
            for target in [meta["turn_target"],"alternate::spelling"]:
                plan=self.plan();plan["injections"].append({"kind":kind,"target":target,"function_fingerprint":meta["turn_fingerprint"]})
                with self.assertRaises(ValueError):guard.check(plan,meta)
    def test_missing_duplicate_source_and_placement_drift_are_rejected(self):
        for change in [lambda p:p.update(modules=[]),lambda p:p["modules"].append(copy.deepcopy(p["modules"][0])),lambda p:p["modules"][0].update(file="other.rs"),lambda p:p["modules"][0].update(source="changed"),lambda p:p.update(inputs={})]:
            plan=self.plan();change(plan)
            with self.assertRaises(ValueError):guard.check(plan,guard.metadata())
    def test_no_automatic_activation_and_single_converter(self):
        manifest=tomllib.loads((NATIVE/"horizon-mixin.toml").read_text())
        for key in ["injection","state"]:self.assertFalse(any(x["id"].startswith("current-book") for x in manifest.get(key,[])))
        source=(NATIVE/"current-book.rs").read_text().split("#[cfg(test)]")[0]
        self.assertIn("self.game.turn_page(",source)
        self.assertIn("pluto_guide_text::from_native(",source)
        for forbidden in ["open_book(","close_book(","send(","Atomic", "static CALLBACK", "page ="]:self.assertNotIn(forbidden,source)
        self.assertLess(source.index("self.invoked = true"),source.index("self.game.turn_page("))
    def test_package_contains_real_api_and_adapter(self):
        spec=importlib.util.spec_from_file_location("book_package",ROOT/"tools/package_native.py")
        package=importlib.util.module_from_spec(spec);spec.loader.exec_module(package)
        _,files=package._bundle(json.loads((ROOT/"horizon-native.json").read_text()))
        for name in ["current-book.rs","inventory-api/src/guide/current_book.rs"]:
            self.assertEqual(files[f"pluto_native_creative/{name}"],(NATIVE/name).read_bytes())
if __name__=="__main__":unittest.main()
