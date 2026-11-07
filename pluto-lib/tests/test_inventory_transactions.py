"""Run the real shared inventory crate, including its dependency-backed APIs."""
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]


class InventoryTransactionTests(unittest.TestCase):
    @unittest.skipUnless(shutil.which("cargo"), "Rust toolchain is required")
    def test_native_inventory_api(self):
        source = ROOT / "mixins/pluto_native_creative/inventory-api"
        with tempfile.TemporaryDirectory(prefix="pluto-inventory-tests-") as temp:
            # Copy the complete real crate, including every module/dependency;
            # isolate Cargo.lock/target writes, not the API under test.
            crate = Path(temp) / "inventory-api"
            shutil.copytree(source, crate, ignore=shutil.ignore_patterns("target", "Cargo.lock"))
            # This standalone test lock is seeded from the selected official
            # host. The native bundle still resolves only against its host lock.
            shutil.copy2(ROOT / "tests/inventory-api.Cargo.lock", crate / "Cargo.lock")
            env = dict(os.environ, CARGO_INCREMENTAL="0", CARGO_PROFILE_DEV_DEBUG="0",
                       CARGO_PROFILE_TEST_DEBUG="0", CARGO_BUILD_JOBS="2")
            env.setdefault("CARGO_TARGET_DIR", str(Path(temp) / "target"))
            if os.environ.get("PLUTO_TEST_FETCH_LOCKED") == "1":
                # CI may have an empty registry cache. Fetch exactly this lock,
                # then keep the actual test offline and immutable.
                subprocess.run(["cargo", "fetch", "--locked", "--manifest-path", str(crate / "Cargo.toml")],
                               env=env, check=True)
            subprocess.run(["cargo", "test", "--offline", "--locked", "--manifest-path", str(crate / "Cargo.toml")],
                           env=env, check=True)


if __name__ == "__main__":
    unittest.main()
