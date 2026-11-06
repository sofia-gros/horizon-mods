"""Small resource/source guards only; never launches native tooling."""
import importlib.util
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

HERE = Path(__file__).resolve().parents[1] / "experiments/terrain-cell-cpu"
sys.dont_write_bytecode = True
sys.path.insert(0, str(HERE))
spec = importlib.util.spec_from_file_location("terrain_tmpfs", HERE / "tmpfs.py")
runner = importlib.util.module_from_spec(spec)
spec.loader.exec_module(runner)


class TmpfsGuards(unittest.TestCase):
    def test_unknown_or_unbounded_cgroup_cannot_use_host_only_admission(self):
        for constraints in ([], [{"limit": None}]):
            with patch.object(runner, "memory_status", return_value={"cgroups": constraints, "effective_available": 99999999999}):
                with self.assertRaisesRegex(ValueError, "not observable"):
                    runner.available_memory()
        with patch.object(runner, "memory_status", return_value={"cgroups": [{"limit": 4096}], "effective_available": 1234}):
            self.assertEqual(runner.available_memory(), 1234)

    def test_frozen_inputs_are_identical_to_published_ci(self):
        for name, expected in runner.FROZEN.items():
            self.assertEqual(runner.ci.digest(HERE / name), expected)

    def test_each_independent_floor_prevents_spawn(self):
        with tempfile.TemporaryDirectory() as directory:
            root, overlay = Path(directory), Path("/unused-overlay")
            for constrained in ("overlay", "tmpfs", "memory"):
                limits = {"overlay": runner.ci.FLOOR, "tmpfs": runner.TMP_FLOOR, "memory": runner.MEM_FLOOR}
                values = {k: v + 1 for k, v in limits.items()}
                values[constrained] = limits[constrained] - 1
                with patch.object(runner, "free", side_effect=lambda path: values["overlay" if path == overlay else "tmpfs"]), patch.object(runner, "available_memory", return_value=values["memory"]):
                    instance = runner.GuardedRunner(root, {}, overlay)
                    with patch.object(runner.ci.subprocess, "Popen") as spawn:
                        with self.assertRaises(ValueError): instance.run("forbidden", ["false"])
                        spawn.assert_not_called()

    def test_registry_seal_detects_contents_not_access_bookkeeping(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory); registry = root / "registry"; registry.mkdir()
            (registry / "Cargo.toml").write_text("original")
            runner.registry_seal(registry, root / "before")
            (root / ".global-cache").write_text("access bookkeeping")
            runner.registry_seal(registry, root / "same")
            self.assertEqual((root / "before").read_bytes(), (root / "same").read_bytes())
            (registry / "Cargo.toml").write_text("changed")
            runner.registry_seal(registry, root / "after")
            self.assertNotEqual((root / "before").read_bytes(), (root / "after").read_bytes())

    def test_registry_symlinks_fail_closed(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory); registry = root / "registry"; registry.mkdir()
            (registry / "link").symlink_to(root / "missing")
            with self.assertRaises(ValueError): runner.registry_seal(registry, root / "out")

    def test_source_guards_and_all_output_controls(self):
        source = (HERE / "tmpfs.py").read_text()
        self.assertIn('CARGO_BUILD_JOBS="1"', source)
        self.assertIn('CARGO_TARGET_DIR=str(target), TMPDIR=str(temp), TMP=str(temp), TEMP=str(temp)', source)
        self.assertIn('CARGO_NET_OFFLINE="true"', source)
        self.assertIn('"cargo", "test", "--offline", "--locked"', source)
        self.assertLess(source.index('runner.run("all-production-guards"'), source.index('runner.run("apply"'))
        self.assertLess(source.index('runner.run("fixture-guard"'), source.index('runner.run("apply"'))
        self.assertIn('before.read_bytes() != after.read_bytes()', source)
        self.assertIn('cache_before.read_bytes() != cache_after.read_bytes()', source)
        self.assertIn('checker.checked_log(log.read_text(), mode)', source)
        self.assertNotIn('shutil.copy', source)
        self.assertNotIn('.symlink_to', source)


if __name__ == "__main__":
    unittest.main()
