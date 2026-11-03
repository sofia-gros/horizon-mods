"""Checks the native archive's source identity and public packaging contract."""
import importlib.util
import shutil
from pathlib import Path
import tempfile
import tomllib
import unittest
import zipfile

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("package_native", ROOT / "tools/package_native.py")
package = importlib.util.module_from_spec(spec)
spec.loader.exec_module(package)


class NativePackageTests(unittest.TestCase):
    def test_combines_components_without_copying_demos_or_registries(self):
        with tempfile.TemporaryDirectory() as temp:
            first, second = Path(temp) / "first.hnmod", Path(temp) / "second.hnmod"
            package.build(first)
            package.build(second)
            self.assertEqual(first.read_bytes(), second.read_bytes())
            with zipfile.ZipFile(first) as archive:
                names = archive.namelist()
                prefix = "mixins/pluto_native_creative/"
                self.assertEqual(sum(name.endswith("horizon-mixin.toml") for name in names), 1)
                self.assertFalse(any("demo" in name or "pluto_creative/" in name for name in names))
                combined = tomllib.loads(archive.read(prefix + "horizon-mixin.toml").decode())
                self.assertEqual(len(combined["crate"]), 2)
                self.assertEqual({entry["path"] for entry in combined["crate"]},
                                 {"pluto_native_creative/api", "pluto_native_creative/inventory-api"})
                item_sources = [name for name in names if name.endswith("/api/src/item.rs")]
                self.assertEqual(len(item_sources), 1)
                self.assertEqual(archive.read(item_sources[0]).count(b"static REGISTRY:"), 1)
                for section in ("crate", "module", "injection", "state"):
                    ids = [entry["id"] for entry in combined.get(section, [])]
                    self.assertEqual(len(ids), len(set(ids)), f"duplicate {section} IDs")
                self.assertIn(prefix + "pluto_native_creative/inventory-api/src/lib.rs", names)
                for component in package.COMPONENTS:
                    root = ROOT / "mixins" / component
                    original = tomllib.loads((root / "horizon-mixin.toml").read_text())
                    for module in original.get("module", []):
                        entry = next(x for x in combined["module"] if x["id"] == f'{component}-{module["id"]}')
                        self.assertEqual(entry["name"], module["name"])
                        self.assertEqual(archive.read(prefix + entry["source"]), (root / module["source"]).read_bytes())
                    for injection in original.get("injection", []):
                        entry = next(x for x in combined["injection"] if x["id"] == f'{component}-{injection["id"]}')
                        self.assertEqual(entry["priority"], injection.get("priority", original["mod"].get("priority", 100)))
                        self.assertEqual(archive.read(prefix + entry["helper"]), (root / injection["helper"]).read_bytes())
                    for state in original.get("state", []):
                        state_id = state["id"] if component == "pluto_native_creative" else f'{component}-{state["id"]}'
                        matches = [x for x in combined.get("state", []) if x["id"] == state_id]
                        self.assertEqual(len(matches), 1)
                        expected = dict(state, id=state_id)
                        self.assertEqual(matches[0], expected)
                        self.assertTrue(matches[0].get("constructor"))
                        self.assertTrue(all(c.get("target") and c.get("selector") and c.get("require", 0) > 0
                                            for c in matches[0]["constructor"]))
                landing = next(x for x in combined["state"] if x["id"] == "landing-state")
                self.assertEqual(landing["target"], "horizon_client_render::chunk_map::ChunkMap")
                self.assertEqual(landing["constructor"], [{"target": "horizon_client_render::chunk_map::<ChunkMap as Default>::default", "selector": "Self", "require": 1}])
                module = next(x for x in combined["module"] if x["name"] == "pluto_landing_state")
                self.assertIn("pluto_landing_state::LandingState", landing["ty"])
                self.assertIn(prefix + module["source"], names)
                self.assertIn(prefix + "NOTICE.md", names)
            with self.assertRaises(FileExistsError):
                package.build(first)

    def test_combines_legacy_components_without_state(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp) / "pluto-lib"
            shutil.copytree(ROOT / "mixins", root / "mixins")
            shutil.copy2(ROOT / "horizon-native.json", root / "horizon-native.json")
            manifest = root / "mixins/pluto_native_creative/horizon-mixin.toml"
            text = manifest.read_text()
            self.assertEqual(text.count('[[state]]'), 1)
            manifest.write_text(text.split('[[state]]', 1)[0])
            previous = package.ROOT
            try:
                package.ROOT = root
                output = Path(temp) / "legacy.hnmod"
                package.build(output)
                with zipfile.ZipFile(output) as archive:
                    prefix = "mixins/pluto_native_creative/"
                    combined = tomllib.loads(archive.read(prefix + "horizon-mixin.toml").decode())
                    self.assertNotIn("state", combined)
                    self.assertEqual(len(combined["crate"]), 2)
                    for section, key in (("module", "source"), ("injection", "helper")):
                        ids = [entry["id"] for entry in combined[section]]
                        self.assertEqual(len(ids), len(set(ids)))
                        for entry in combined[section]:
                            self.assertIn(prefix + entry[key], archive.namelist())
            finally:
                package.ROOT = previous


if __name__ == "__main__":
    unittest.main()
