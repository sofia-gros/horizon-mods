"""Source-only guards for the additive, same-owner inventory public entry."""
import hashlib
import importlib.util
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]
API = ROOT / "mixins/pluto_native_creative/inventory-api"


class InventoryPublicEntryTests(unittest.TestCase):
    def test_entry_is_only_reexports_before_its_tests(self):
        text = (API / "src/inventory.rs").read_text()
        source = text.split("#[cfg(test)]", 1)[0]
        lines = [line.strip() for line in source.splitlines()
                 if line.strip() and not line.lstrip().startswith("//")]
        self.assertEqual(lines, [
            "pub use crate::{plan_between, plan_move, MoveError, MovePlan, StackSnapshot};",
            "pub use crate::storage::stage_updates;",
        ])
        self.assertEqual((API / "src/lib.rs").read_text().count("pub mod inventory;"), 1)

    def test_existing_planning_and_staging_implementation_bytes_are_unchanged(self):
        expected = {
            "lib.rs": "06715bb97fc161495298fb187e39e903d28027f8c6904c88c432e7e839761a94",
            "storage.rs": "8cacb210eaa31b3e7d2fe028f4f1171f3707ed9f1e06dca528728a3b54c64117",
        }
        for name, digest in expected.items():
            source = (API / "src" / name).read_text().split("#[cfg(test)]", 1)[0]
            self.assertEqual(hashlib.sha256(source.encode()).hexdigest(), digest, name)

    def test_legacy_example_is_frozen_and_new_example_only_changes_import(self):
        old = (ROOT / "examples/inventory/stage_proposals.rs").read_bytes()
        self.assertEqual(hashlib.sha256(old).hexdigest(),
                         "76ba215f7594a949df262deb699790eae73c6c946051f43d85d0816911c51ae4")
        before = "use pluto_inventory_api::{plan_between, storage::stage_updates, MoveError, StackSnapshot};"
        after = "use pluto_inventory_api::inventory::{plan_between, stage_updates, MoveError, StackSnapshot};"
        self.assertEqual(old.decode().count(before), 1)
        self.assertEqual((ROOT / "examples/inventory/stage_proposals_public.rs").read_text(),
                         old.decode().replace(before, after))

    def test_package_crate_discovery_keeps_one_complete_source(self):
        spec = importlib.util.spec_from_file_location("inventory_entry_package", ROOT / "tools/package_native.py")
        package = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(package)
        files = package._crate_sources(API.parent, "inventory-api")
        path = "inventory-api/src/inventory.rs"
        self.assertEqual(files[path], (API / "src/inventory.rs").read_bytes())
        self.assertEqual(sum(name.endswith("/src/inventory.rs") for name in files), 1)
        self.assertEqual((API.parent / "inventory-export.rs").read_text(),
                         "// One shared instance, re-exported by horizon-client through its client library.\n"
                         "pub use pluto_inventory_api::*;\n")


if __name__ == "__main__":
    unittest.main()
