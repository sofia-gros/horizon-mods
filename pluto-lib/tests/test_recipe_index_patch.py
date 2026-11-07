"""Source/package boundaries, separate from the exact-index behavior fixture."""
from pathlib import Path
import importlib.util
import tempfile
import tomllib
import unittest
import zipfile

ROOT = Path(__file__).resolve().parents[1]
NATIVE = ROOT / "mixins/pluto_native_creative"
spec = importlib.util.spec_from_file_location("index_patch_package", ROOT / "tools/package_native.py")
package = importlib.util.module_from_spec(spec)
spec.loader.exec_module(package)


class IndexPatchBoundaries(unittest.TestCase):
    def test_only_explicit_native_support_module(self):
        manifest = tomllib.loads((NATIVE / "horizon-mixin.toml").read_text())
        modules = [m for m in manifest["module"] if m["id"] == "recipe-index-patch"]
        self.assertEqual(len(modules), 1)
        self.assertEqual(modules[0]["file"], "crates/horizon-client/src/recipe_book.rs")
        self.assertEqual(modules[0]["name"], "pluto_index_patch")
        for kind in ("injection", "state", "crate"):
            self.assertFalse(any(m["id"] == "recipe-index-patch" for m in manifest.get(kind, [])))

    def test_policy_and_owner_identity_remain_outside(self):
        api = (NATIVE / "inventory-api/src/recipe_book/index.rs").read_text()
        native = (NATIVE / "recipe-index-patch.rs").read_text()
        for banned in ("horizon_", "tconstruct", "static REGISTRY", "ptr_eq", "AtomicU64"):
            self.assertNotIn(banned, api)
        for banned in ("starts_with(", "static REGISTRY", "&mut Game", "world_tables", ".send(", "Box::leak", ".level.recipe_book"):
            self.assertNotIn(banned, native)
        self.assertIn("Allocation::Append", api)
        self.assertIn("Allocation::FirstGap", api)

    def test_single_mutation_after_all_fallible_validation(self):
        source = (NATIVE / "recipe-index-patch.rs").read_text()
        commit = "self.0.recipes = candidate;"
        self.assertEqual(source.count(commit), 1)
        tail = source.split(commit, 1)[1].split("pub fn with_index", 1)[0]
        self.assertNotIn("?", tail)
        self.assertIn("station(&record.station)?", source)
        self.assertIn("index::plan(&current, patch)?", source)

    def test_archive_has_the_single_api_and_native_adapter(self):
        with tempfile.TemporaryDirectory() as tmp:
            output = Path(tmp) / "index.hnmod"
            package.build(output)
            with zipfile.ZipFile(output) as archive:
                prefix = "mixins/pluto_native_creative/pluto_native_creative/"
                for name in ("recipe-index-patch.rs", "inventory-api/src/recipe_book/index.rs"):
                    self.assertEqual(archive.read(prefix + name), (NATIVE / name).read_bytes())


if __name__ == "__main__":
    unittest.main()
