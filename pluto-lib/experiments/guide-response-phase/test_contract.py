"""Research/source regressions, not Rust codec, native type or live UI proof.

Run with PYTHONDONTWRITEBYTECODE=1; packaging is inspected only in memory.
"""
import hashlib
import importlib.util
import json
from pathlib import Path
import re
import tomllib
import unittest

HERE = Path(__file__).resolve().parent
PLUTO = HERE.parents[1]
MODS = PLUTO.parent
NATIVE = PLUTO / "mixins/pluto_native_creative"
CONSUMER = MODS / "tconstruct/mixins/tconstruct"


def compact(value):
    return json.dumps(value, ensure_ascii=False, separators=(",", ":")).encode()


def legacy_span(text, insertion=None):
    return dict(text=text, page=None, insertion=insertion, color=None, bold=None,
                italic=None, underlined=None, strikethrough=None, obfuscated=None)


class ResponseConstraintTests(unittest.TestCase):
    def test_probes_are_exact_no_op_single_hooks_without_state(self):
        sources = json.loads((HERE / "sources.json").read_text())
        for path, expected in sources["probe_sources"].items():
            self.assertEqual(hashlib.sha256((HERE / path).read_bytes()).hexdigest(), expected)
        for name, kind in [("return-mut", "RETURN"), ("after-call", "AFTER_CALL")]:
            root = HERE / name / "pluto_guide_response_probe"
            manifest = tomllib.loads((root / "horizon-mixin.toml").read_text())
            self.assertFalse(set(manifest) & {"module", "crate", "state"})
            self.assertEqual(len(manifest["injection"]), 1)
            hook = manifest["injection"][0]
            self.assertEqual(hook["kind"], kind)
            self.assertFalse(hook.get("cancellable", False))
            body = (root / hook["helper"]).read_text()
            self.assertIn("let _ = (game, packet);", body)
            self.assertNotRegex(body, r"(?:send|observe|open_book|register|clone)\(")

    def test_record_separates_acceptance_rejection_and_unrun_stages(self):
        proof = json.loads((HERE / "validation.json").read_text())
        self.assertEqual([(p["name"], p["exit_code"], p["plan_written"])
                          for p in proof["phases"]],
                         [("return-mut", 0, True), ("after-call", 1, False)])
        self.assertEqual(proof["rejection"]["code"], "E_UNSUPPORTED_CONTROL_FLOW")
        hook = proof["accepted_injection"]
        self.assertEqual(hook["kind"], "RETURN")
        self.assertEqual(hook["bindings"][1],
                         {"name": "game", "ty": "&mut Game<'_>", "value": "&mut *self"})
        self.assertGreaterEqual(proof["minimum_free_bytes"], proof["floor_bytes"])
        self.assertFalse(proof["disk_stopped"])
        self.assertIn("Real Client UI/lifetime/network", proof["unrun"])

    def test_experiment_is_not_in_actual_in_memory_bundle(self):
        spec = importlib.util.spec_from_file_location("response_package", PLUTO / "tools/package_native.py")
        package = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(package)
        manifest, files = package._bundle(json.loads((PLUTO / "horizon-native.json").read_text()))
        self.assertNotIn(b"pluto_guide_response_probe", manifest)
        self.assertFalse(any("guide-response-phase" in p for p in files))
        for body in files.values():
            self.assertNotIn(b"inspect_post_observer", body)
            self.assertNotIn(b"inspect_caller_after_observer", body)
            self.assertNotIn(b"struct RetainedSlot", body)
            self.assertNotIn(b"struct LegacyU64PrefixV1", body)

    def test_retained_model_reuses_actual_correlation_without_installing_hooks(self):
        source = (HERE / "retained_model.rs").read_text()
        for path in ["inventory-api/src/client_payload.rs", "inventory-api/src/guide/requests.rs"]:
            self.assertIn(path, source)
        self.assertIn("use requests::{RequestId, RequestTracker};", source)
        self.assertIn("self.tracker.consume(", source)
        self.assertIn("client_payload::MAX_BYTES.checked_add(HEADER)", source)
        production = source.split("#[cfg(test)]")[0]
        self.assertNotIn("fn register(", production)
        self.assertNotIn("game.", production)
        self.assertNotIn(".send(", production)

    def test_installed_observer_stays_read_only_and_exclusive(self):
        meta = json.loads((PLUTO / "tools/client-payload-source.json").read_text())
        self.assertEqual(meta["bindings"][1],
                         {"name": "game", "ty": "&Game<'_>", "value": "&*self"})
        helper = (NATIVE / meta["helper"]).read_bytes()
        self.assertEqual(hashlib.sha256(helper).hexdigest(), meta["helper_sha256"])
        guard = (PLUTO / "tools/check_client_payload_plan.py").read_text()
        self.assertIn("len(own) != 1 or len(selected) != 1", guard)
        manifest = tomllib.loads((NATIVE / "horizon-mixin.toml").read_text())
        hooks = [h for h in manifest["injection"] if h["id"].startswith("client-payload")]
        self.assertEqual(len(hooks), 1)
        self.assertEqual(hooks[0]["bindings"], meta["bindings"])

    def test_legacy_envelope_exceeds_unchanged_observer_ceiling(self):
        wire = (CONSUMER / "source-materials-book-wire.rs").read_text()
        observation = (NATIVE / "inventory-api/src/client_payload.rs").read_text()
        limit = int(re.search(r"const LIMIT: usize = ([\d_]+);", wire)[1].replace("_", ""))
        cap = int(re.search(r"const MAX_BYTES: usize = ([\d_]+);", observation)[1].replace("_", ""))
        self.assertEqual(limit, cap)
        self.assertIn("result.extend_from_slice(&sequence.to_be_bytes());", wire)
        self.assertIn("let body = encode(pages)?;", wire)
        # Exact JSON-shape/length model only, not execution of the Rust encoder.
        pages = [[legacy_span("Puny Smelting\n", "tconstruct:materials-book-root")]]
        while len(compact(pages + [[legacy_span("x" * 4000)]])) <= limit:
            pages.append([legacy_span("x" * 4000)])
        pages.append([legacy_span("")])
        size = limit - len(compact(pages))
        self.assertTrue(0 <= size <= 4000)
        pages[-1][0]["text"] = "x" * size
        self.assertEqual(len(compact(pages)), limit)
        self.assertEqual(len((42).to_bytes(8, "big") + compact(pages)), cap + 8)

    def test_sparse_legacy_json_and_canonical_document_budgets_are_distinct(self):
        # Alternating styles avoid assuming native adjacent-style merging. This
        # model does not claim a native Text/decoder execution or server output.
        pages = [[dict(text="x", bold=bool(j % 2)) for j in range(20)] for _ in range(511)]
        pages[0][0].update(text="Puny Smelting\n", insertion="tconstruct:materials-book-root")
        legacy = compact(pages)
        canonical = [[dict(text=s["text"], color=None, bold=s["bold"], italic=None,
                           underlined=None, strikethrough=None, obfuscated=None,
                           page=None, metadata=s.get("insertion")) for s in p] for p in pages]
        document = compact(dict(version=1, pages=canonical))
        self.assertEqual(len(legacy), 261692)
        self.assertEqual(len(document), 1406337)
        self.assertLessEqual(len(legacy), 262144)
        self.assertGreater(len(document), 262144)
        source = (NATIVE / "inventory-api/src/guide/document.rs").read_text()
        self.assertIn("encode(&document)?;", source)

    def test_lan_response_and_cover_behavior_remain_active_unchanged(self):
        manifest = tomllib.loads((CONSUMER / "horizon-mixin.toml").read_text())
        response = next(h for h in manifest["injection"] if h["id"] == "materials-book-response")
        self.assertEqual(response["kind"], "HEAD")
        self.assertTrue(response["cancellable"])
        source = (CONSUMER / response["helper"]).read_text()
        self.assertIn("BookScreen::View { pages, page, lectern: None }", source)
        self.assertIn("if *page != usize::MAX", source)
        self.assertIn("*pages = decoded;", source)
        self.assertNotIn("pluto_current_book", source)
        self.assertNotIn("dialog_open", source)
        # This intentionally records the remaining conflict, not its resolution.


if __name__ == "__main__":
    unittest.main()
