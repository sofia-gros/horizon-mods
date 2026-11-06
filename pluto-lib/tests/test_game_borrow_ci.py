"""Small source/diagnostic contracts only; no Cargo, native tools or dispatch."""
import copy
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
import zipfile

ROOT = Path(__file__).resolve().parents[1]
HERE = ROOT / "experiments/game-borrow-ci"
spec = importlib.util.spec_from_file_location("borrow_ci", HERE / "ci.py")
ci = importlib.util.module_from_spec(spec)
spec.loader.exec_module(ci)


class GameBorrowCiChecks(unittest.TestCase):
    def event(self):
        return {"repository": {"full_name": "sofia-gros/horizon-mods", "private": False},
            "inputs": {"game_borrow": "true"}}

    def env(self):
        return {"GITHUB_ACTIONS": "true", "GITHUB_EVENT_NAME": "workflow_dispatch",
            "GITHUB_REPOSITORY": "sofia-gros/horizon-mods", "RUNNER_OS": "Linux", "GITHUB_SHA": "a" * 40}

    def test_modes_are_explicit_and_independent(self):
        ci.shared.admit(self.env(), self.event(), "game_borrow")
        with self.assertRaises(ValueError): ci.shared.admit(self.env(), self.event())
        for name in ["unknown", "", "game-borrow"]:
            with self.assertRaises(ValueError): ci.shared.admit(self.env(), self.event(), name)
        for private in [True, None]:
            event = self.event(); event["repository"]["private"] = private
            with self.assertRaises(ValueError): ci.shared.admit(self.env(), event, "game_borrow")

    def test_exact_frozen_probe_bytes_and_v15_target(self):
        self.assertEqual(ci.shared.BUNDLE_SHA, "7d9c4c5d7bd91ec6584b9206658f9d5bdb5d04dd3f95f1cfe90ceca6daae5386")
        self.assertEqual(ci.digest_bytes(ci.checked_probe("positive")), "2199396795b3c26b94f618bbb243b00ed980705c45dce7028d807158a9518b1e")
        self.assertEqual(ci.digest_bytes(ci.checked_probe("negative")), "5033febfe81224d4582f59a41bf470a35c19e5452c201d4314461b400032d01f")
        with self.assertRaises(ValueError): ci.checked_probe("observer")
        with self.assertRaises(ValueError): ci.expected_overlay({}, b"not actual Game", "positive")

    def diagnostic(self):
        # Independent small Cargo diagnostic, not output from a native compiler.
        body = b"// existing code\nmod pluto_contact_negative_probe {\n        game.add_player_effect(effect);\n}\n"
        error = {"reason": "compiler-message", "target": {"name": "horizon_client", "kind": ["lib"]},
            "message": {"level": "error", "code": {"code": "E0499"}, "spans": [{
                "is_primary": True, "file_name": "crates/horizon-client/src/game.rs", "line_start": 3,
                "line_end": 3, "byte_start": 61, "byte_end": 84,
                "text": [{"text": "        game.add_player_effect(effect);"}]}]}}
        return body, error

    def test_negative_requires_one_specific_real_target_error(self):
        body, error = self.diagnostic()
        self.assertEqual(ci.checks.reject_diagnostic(json.dumps(error), body)["code"], "E0499")
        for wrong in ["E0308", None]:
            altered = copy.deepcopy(error); altered["message"]["code"] = {"code": wrong}
            with self.assertRaises(ValueError): ci.checks.reject_diagnostic(json.dumps(altered), body)
        for kind in [["bin"], ["test"]]:
            altered = copy.deepcopy(error); altered["target"]["kind"] = kind
            with self.assertRaises(ValueError): ci.checks.reject_diagnostic(json.dumps(altered), body)
        with self.assertRaises(ValueError): ci.checks.reject_diagnostic(json.dumps(error) + "\n" + json.dumps(error), body)
        with self.assertRaises(ValueError): ci.checks.reject_diagnostic("", body)
        with self.assertRaises(ValueError): ci.checks.reject_diagnostic("not Cargo JSON", body)

    def test_negative_rejects_wrong_call_and_module_span(self):
        body, error = self.diagnostic()
        for field, value in [("file_name", "unrelated.rs"), ("line_start", 1), ("byte_start", 0),
                ("byte_end", len(body) + 1), ("is_primary", False)]:
            altered = copy.deepcopy(error); altered["message"]["spans"][0][field] = value
            with self.assertRaises(ValueError): ci.checks.reject_diagnostic(json.dumps(altered), body)

    def test_only_one_exact_spanless_abort_summary_is_tolerated(self):
        body, error = self.diagnostic()
        summary = {"reason": "compiler-message", "message": {"level": "error", "code": None,
            "spans": [], "message": "aborting due to 1 previous error"}}
        lines = json.dumps(error) + "\n" + json.dumps(summary)
        self.assertEqual(ci.checks.reject_diagnostic(lines, body)["optional_abort_summary"], summary)
        with self.assertRaises(ValueError): ci.checks.reject_diagnostic(lines + "\n" + json.dumps(summary), body)
        for field, value in [("message", "aborting due to 2 previous errors"), ("code", {"code": "E0308"}),
                ("spans", [{"is_primary": True}])]:
            altered = copy.deepcopy(summary); altered["message"][field] = value
            with self.assertRaises(ValueError):
                ci.checks.reject_diagnostic(json.dumps(error) + "\n" + json.dumps(altered), body)

    def test_capacity_rejects_before_compiler_start(self):
        with tempfile.TemporaryDirectory() as directory:
            with patch.object(ci.Runner, "free", return_value=ci.shared.FLOOR - 1):
                runner = ci.Runner(Path(directory), {})
                with patch.object(ci.subprocess, "Popen") as spawn:
                    with self.assertRaises(ValueError): runner.check("negative", ["false"], Path(directory), 101)
                    spawn.assert_not_called()

    def test_package_extraction_rejects_traversal_and_keeps_exact_members(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory); bundle = root / "package.zip"
            for number, relative in enumerate(["helper.rs", "../../escape.rs"]):
                with zipfile.ZipFile(bundle, "w") as archive:
                    archive.writestr("mixins/pluto_native_creative/horizon-mixin.toml", "manifest")
                    archive.writestr("mixins/pluto_native_creative/" + relative, "helper")
                with patch.object(ci.shared, "BUNDLE_SHA", ci.shared.digest(bundle)):
                    if number:
                        with self.assertRaises(ValueError): ci.unpack(bundle, root / str(number))
                        self.assertFalse((root / str(number)).exists())
                    else:
                        ci.unpack(bundle, root / str(number))
                        self.assertEqual((root / str(number) / "pluto_native_creative/helper.rs").read_text(), "helper")

    def test_native_generated_inputs_and_stage_drift_are_not_silently_ignored(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory); output = root / "debug/build/horizon-client-abc/out"
            output.mkdir(parents=True); (output / "generated.rs").write_text("first")
            before = ci.generated_inputs(root)
            (output / "generated.rs").write_text("second")
            self.assertNotEqual(before, ci.generated_inputs(root))
            expected = ci.file_map(root); (root / "unexpected.rs").write_text("extra")
            with self.assertRaises(ValueError): ci.verify_map(root, expected)

    def test_workflow_is_manual_standard_read_only_and_no_storage_actions(self):
        workflow = (ROOT.parent / ".github/workflows/mixins.yml").read_text()
        job = workflow.split("\n  game-borrow:\n", 1)[1]
        for text in ["runs-on: ubuntu-24.04", "permissions:\n      contents: read", "persist-credentials: false",
                "github.event.repository.private == false", "inputs.game_borrow == true", "timeout-minutes: 60"]:
            self.assertIn(text, job)
        for text in ["actions/cache@", "actions/upload-artifact@", "secrets.", ": write", "-large", "self-hosted"]:
            self.assertNotIn(text, job)
        self.assertEqual(job.count("uses:"), 1)
        self.assertIn("inputs.terrain_cpu != true && inputs.game_borrow != true", workflow)

    def test_production_guards_and_positive_precede_negative(self):
        source = (HERE / "ci.py").read_text()
        self.assertLess(source.index('runner.run("all-production-guards"'), source.index('runner.run("official-apply"'))
        self.assertIn('for kind in ("positive", "negative"):', source)
        self.assertIn('0 if kind == "positive" else 101', source)
        self.assertIn('checks.reject_diagnostic(messages, content)', source)
        self.assertIn('if current_generated != generated:', source)
        self.assertIn('if before.read_bytes() != after.read_bytes()', source)
        self.assertIn('"runtime_executed": False, "gpu_executed": False', source)


if __name__ == "__main__":
    unittest.main()
