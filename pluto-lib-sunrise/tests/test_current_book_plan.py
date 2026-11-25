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
        manifest=tomllib.loads((NATIVE/"sunrise-mod.toml").read_text())
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
class NativeCurrentBookTests(unittest.TestCase):
    def test_source_and_composition_fail_closed(self):
        import hashlib
        import tempfile
        from unittest.mock import patch
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / guard.NATIVE_FILE
            source.parent.mkdir(parents=True)
            source.write_bytes(b'native books')
            expected = hashlib.sha256(source.read_bytes()).hexdigest()
            with patch.object(guard, 'NATIVE_SHA256', expected):
                plan = dict(schema=6, inputs={guard.NATIVE_FILE: expected}, injections=[],
                            states=[], modules=[{'mod_id': 'pluto_lib', 'file': 'unrelated.rs'}])
                meta = guard.metadata(plan)
                guard.check(plan, meta)
                guard.check_source(root, meta)
                for section in ('injections', 'modules', 'states'):
                    for entry in ({'file': guard.NATIVE_FILE}, {'target': 'alias::Game::turn_page'},
                                  {'mod_id': 'pluto_native_creative'}):
                        plan[section].append(entry)
                        with self.assertRaisesRegex(ValueError, 'unattached'):
                            guard.check(plan, meta)
                        plan[section].pop()
                plan['inputs'].clear()
                with self.assertRaisesRegex(ValueError, 'plan source differs'):
                    guard.check(plan, meta)
                plan['inputs'][guard.NATIVE_FILE] = expected
                with self.assertRaisesRegex(ValueError, 'metadata'):
                    guard.check(plan, guard.metadata())
                plan['schema'] = 7
                with self.assertRaisesRegex(ValueError, 'schema'):
                    guard.check(plan, meta)
                source.write_bytes(b'changed')
                with self.assertRaisesRegex(ValueError, 'source drifted'):
                    guard.check_source(root, meta)


if __name__=="__main__":unittest.main()
