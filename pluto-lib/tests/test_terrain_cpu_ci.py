"""Small CI admission/source checks; no native build, plan or remote dispatch."""
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[1]
HERE = ROOT / "experiments/terrain-cell-cpu"
spec = importlib.util.spec_from_file_location("terrain_ci", HERE / "ci.py")
ci = importlib.util.module_from_spec(spec)
spec.loader.exec_module(ci)


class TerrainCpuCiChecks(unittest.TestCase):
    def event(self):
        return {"repository": {"full_name": "sofia-gros/horizon-mods", "private": False},
            "inputs": {"terrain_cpu": "true"}}

    def env(self):
        return {"GITHUB_ACTIONS": "true", "GITHUB_EVENT_NAME": "workflow_dispatch",
            "GITHUB_REPOSITORY": "sofia-gros/horizon-mods", "RUNNER_OS": "Linux",
            "GITHUB_SHA": "a" * 40}

    def test_admission_rejects_private_automatic_foreign_and_flagged_runs(self):
        ci.admit(self.env(), self.event())
        for key, value in [("GITHUB_EVENT_NAME", "push"), ("GITHUB_REPOSITORY", "someone/else"),
                ("RUNNER_OS", "Windows"), ("GITHUB_SHA", "main"), ("RUSTFLAGS", "-O")]:
            with self.assertRaises(ValueError):
                ci.admit(self.env() | {key: value}, self.event())
        for private in [True, None, "false"]:
            event = self.event(); event["repository"]["private"] = private
            with self.assertRaises(ValueError): ci.admit(self.env(), event)
        for requested in [None, "false", False]:
            event = self.event(); event["inputs"]["terrain_cpu"] = requested
            with self.assertRaises(ValueError): ci.admit(self.env(), event)

    def test_floor_rejects_before_a_subprocess_starts(self):
        with tempfile.TemporaryDirectory() as directory:
            with patch.object(ci.Runner, "free", return_value=ci.FLOOR - 1):
                runner = ci.Runner(Path(directory), {})
                with patch.object(ci.subprocess, "Popen") as spawn:
                    with self.assertRaises(ValueError): runner.run("must-not-run", ["false"])
                    spawn.assert_not_called()

    def test_all_subprocesses_have_process_group_cleanup(self):
        source = (HERE / "ci.py").read_text()
        self.assertIn("start_new_session=True", source)
        self.assertIn("finally:\n            if process is not None:\n                terminate(process)", source)
        self.assertIn("os.killpg(process.pid, signal.SIGINT)", source)
        self.assertIn("os.killpg(process.pid, signal.SIGKILL)", source)
        self.assertIn("time.sleep(0.25)", source)

    def test_only_exact_official_sources_and_v14_archive_are_admitted(self):
        self.assertEqual(ci.OFFICIAL, {
            "horizon": ("https://github.com/horizon-rust/horizon.git", "624c4f6265c02296fbe8a80f029dd0d987fb94c0"),
            "mixins": ("https://github.com/horizon-rust/horizon-mixins.git", "e8cd0aa38cffef49cb2e088ef716baa107148c02")})
        self.assertEqual(ci.BUNDLE_SHA, "80adf5750e59abbaf3060a57025610230eb3cab73452761b0763699d03bbd6a2")
        source = (HERE / "ci.py").read_text()
        self.assertIn('if digest(bundle) != BUNDLE_SHA:', source)
        self.assertIn('if identity.read_text().strip() != revision:', source)

    def test_guards_precede_apply_and_exact_non_skipping_tests_follow(self):
        source = (HERE / "ci.py").read_text()
        self.assertLess(source.index('runner.run("all-production-guards"'), source.index('runner.run("official-apply"'))
        self.assertLess(source.index('runner.run("fixture-plan-guard"'), source.index('runner.run("official-apply"'))
        self.assertIn('for mode in ("no-provider", "registered"):', source)
        self.assertIn('checker.checked_log(log.read_text(), mode)', source)
        self.assertIn('"--exact", "--nocapture", "--test-threads=1"', source)
        self.assertLess(source.index('runner.run("seal-applied-stage"'), source.index('for mode in ("no-provider", "registered")'))
        self.assertLess(source.index('runner.run("verify-fixture-sources-after"'), source.index('result["status"] = "passed"'))
        self.assertIn('if before.read_bytes() != after.read_bytes():', source)

    def test_stage_seal_detects_lock_source_and_new_file_changes(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory); stage = root / "stage"; stage.mkdir()
            (stage / "Cargo.toml").write_text("manifest")
            (stage / "Cargo.lock").write_text("lock")
            (stage / "source.rs").write_text("source")
            before = root / "before.json"; ci.seal(stage, before)
            same = root / "same.json"; ci.seal(stage, same)
            self.assertEqual(before.read_bytes(), same.read_bytes())
            for number, name in enumerate(["Cargo.lock", "source.rs", "extra.rs"]):
                (stage / name).write_text("changed")
                after = root / f"after-{number}.json"; ci.seal(stage, after)
                self.assertNotEqual(before.read_bytes(), after.read_bytes())

    def test_manual_job_has_read_only_standard_runner_and_no_storage_actions(self):
        workflow = (ROOT.parent / ".github/workflows/mixins.yml").read_text()
        fixture = workflow.split("\n  terrain-cpu:\n", 1)[1]
        self.assertIn("runs-on: ubuntu-24.04", fixture)
        self.assertIn("timeout-minutes: 60", fixture)
        self.assertIn("permissions:\n      contents: read", fixture)
        self.assertIn("persist-credentials: false", fixture)
        self.assertIn("github.event.repository.private == false", fixture)
        self.assertIn("github.event_name == 'workflow_dispatch'", fixture)
        for forbidden in ["actions/cache@", "actions/upload-artifact@", "secrets.", ": write", "-large", "self-hosted", "snapshot:"]:
            self.assertNotIn(forbidden, fixture)
        self.assertEqual(fixture.count("uses:"), 1)
        self.assertIn("github.event_name != 'workflow_dispatch' || inputs.terrain_cpu != true", workflow)

    def test_production_bytes_still_match_the_frozen_cell_guard(self):
        import hashlib
        metadata = json.loads((ROOT / "tools/terrain-cell-source.json").read_text())
        native = ROOT / "mixins/pluto_native_creative"
        for entry in metadata["modules"]:
            self.assertEqual(hashlib.sha256((native / entry["source"]).read_bytes()).hexdigest(), entry["sha256"])


if __name__ == "__main__":
    unittest.main()
