"""Checks the native archive's source identity and public packaging contract."""
import importlib.util
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
                self.assertEqual(len(combined["crate"]), 1)
                self.assertEqual(combined["crate"][0]["path"], "pluto_native_creative/api")
                item_sources = [name for name in names if name.endswith("/api/src/item.rs")]
                self.assertEqual(len(item_sources), 1)
                self.assertEqual(archive.read(item_sources[0]).count(b"static REGISTRY:"), 1)
                self.assertNotIn("state", combined)
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
                self.assertIn(prefix + "NOTICE.md", names)
            with self.assertRaises(FileExistsError):
                package.build(first)


if __name__ == "__main__":
    unittest.main()
