"""Read-only source/oracle checks, not an actual packet, Rust or GUI fixture."""
import copy
import importlib.util
import json
import os
from pathlib import Path
import re
import tomllib
import unittest

import render

ROOT = Path(os.environ["GUIDE_BRIDGE_INPUTS"])
MANIFEST = Path(os.environ["GUIDE_TC_MANIFEST"])
OFFICIAL = Path(os.environ["GUIDE_BRIDGE_OFFICIAL"])
META = json.loads((render.HERE / "inputs.json").read_text())


class SourceTests(unittest.TestCase):
    def setUp(self):
        self.before = render.inputs(ROOT, MANIFEST)
        self.after = render.render(self.before)

    def test_all_input_and_candidate_pins(self):
        self.assertEqual(len(META["candidate_sha256"]), 4)
        self.assertEqual(len(self.after), 7)

    def test_full_inverse_and_only_expected_injection_changes(self):
        for prefix, target in [(render.NATIVE, "client-payload-observed"), (render.CONSUMER, "materials-book-response")]:
            old = tomllib.loads(self.before[prefix + "horizon-mixin.toml"])
            new = tomllib.loads(self.after[prefix + "horizon-mixin.toml"])
            self.assertEqual([h for h in old["injection"] if h["id"] != target], [h for h in new["injection"] if h["id"] != target])
            own = [h for h in new["injection"] if h["id"] == target]
            self.assertEqual(len(own), 1)
            self.assertFalse(own[0].get("cancellable", False))
            self.assertEqual(own[0]["kind"], "RETURN" if prefix == render.NATIVE else "HEAD")
        # render itself checks both full manifest and full book inverse equality.

    def test_existing_guard_explicitly_allows_only_noncancellable_bootstrap(self):
        path = ROOT / "pluto-lib/tools/check_client_payload_plan.py"
        spec = importlib.util.spec_from_file_location("existing_guard", path)
        guard = importlib.util.module_from_spec(spec); spec.loader.exec_module(guard)
        meta = guard.metadata()
        base = {"inputs": meta["source_sha256"], "injections": [{
            "mod_id": "pluto_native_creative", "injection_id": meta["injection_id"],
            "target": meta["target"], **meta["location"], "kind": "RETURN", "cancellable": False,
            "function_fingerprint": meta["function_fingerprint"], "bindings": meta["bindings"],
            "helper_source": self.before[render.NATIVE + "client-payload-observed.rs"],
        }]}
        bootstrap = tomllib.loads(self.after[render.CONSUMER + "horizon-mixin.toml"])["injection"]
        bootstrap = next(x for x in bootstrap if x["id"] == "materials-book-response")
        plan = copy.deepcopy(base); plan["injections"].append(bootstrap)
        guard.check(plan, meta)
        for change in ({"cancellable": True}, {"kind": "MODIFY_VARIABLE"}):
            bad = copy.deepcopy(plan); bad["injections"][-1].update(change)
            with self.assertRaises(ValueError): guard.check(bad, meta)
        second = copy.deepcopy(base); second["injections"].append(copy.deepcopy(base["injections"][0]))
        with self.assertRaises(ValueError): guard.check(second, meta)
        changed = copy.deepcopy(base); changed["injections"][0]["helper_source"] = (render.HERE / "after-native-packet.rs").read_text()
        with self.assertRaises(ValueError): guard.check(changed, meta)

    def test_bootstrap_has_no_update_or_early_native_return(self):
        raw = (render.HERE / "bootstrap.rs").read_text()
        self.assertIn('message.channel != "tconstruct:materials_book"', raw)
        self.assertEqual(raw.count("ensure_registered(game)"), 1)
        for word in ["game.book", "request_sequence", "decode_reply", "replace_pages", "ControlFlow", "Break(", "send("]:
            self.assertNotIn(word, raw)

    def test_open_registers_before_every_old_effect_and_restores_exact_body(self):
        raw = self.after[render.CONSUMER + "source-materials-book.rs"]
        fn = raw[raw.index("pub(super) fn open("):]
        registration = fn.index("ensure_registered(game)")
        for effect in ["source_pages(", "wrapping_add(1)", "_history_mut()", "game.open_book(", "game.send("]:
            self.assertLess(registration, fn.index(effect))
        self.assertEqual(render.once(raw, render.OPEN_AFTER, render.OPEN_BEFORE), self.before[render.CONSUMER + "source-materials-book.rs"])

    def test_registered_callbacks_are_outside_lock_and_after_native_observer(self):
        helper = (render.HERE / "after-native-packet.rs").read_text()
        self.assertLess(helper.index("::snapshot(packet)"), helper.index("::observe(packet)"))
        self.assertLess(helper.index("::observe(packet)"), helper.index("snapshot.dispatch(game)"))
        raw = (render.HERE / "native-bridge.rs").read_text().split("#[cfg(test)]")[0]
        self.assertLess(raw.index("let context = (self.handler.context)(game)"), raw.index("game.book.as_mut()"))
        self.assertEqual(raw.count("(self.handler.update)("), 1)
        self.assertNotIn("native_client_packet(", helper)
        self.assertNotIn("tconstruct:", raw)
        self.assertNotIn("sequence", "\n".join(line for line in raw.splitlines() if not line.lstrip().startswith("//")))
        self.assertNotIn("system_message", helper)

    def test_failure_notification_is_bounded_and_claimed_before_callback(self):
        raw = (render.HERE / "consumer.rs").read_text()
        self.assertIn("REGISTERED.get_or_init(|| register(&RESPONSE))", raw)
        self.assertLess(raw.index("FAILURE_REPORTED.swap(true"), raw.index("game.system_message("))
        self.assertIn("pub(in crate::game) fn ensure_registered", raw)
        self.assertNotIn("pub(super) fn ensure_registered", raw)
        self.assertEqual(raw.count("game.system_message("), 1)
        self.assertEqual(raw.count("decode_reply("), 1)
        self.assertEqual(raw.count("view.replace_pages("), 1)
        self.assertIn("max_bytes: 262_152", raw)

    def test_request_codec_and_sequence_policy_remain_original(self):
        self.assertNotIn(render.CONSUMER + "source-materials-book-wire.rs", self.after)
        self.assertNotIn(render.CONSUMER + "source-materials-book-request.rs", self.after)
        request = self.before[render.CONSUMER + "source-materials-book-request.rs"]
        self.assertEqual(request.count("guest.send("), 1)
        self.assertIn("[hand @ (0 | 1)]", request)
        self.assertIn("bytes.len() == 8", request)
        wire = self.before[render.CONSUMER + "source-materials-book-wire.rs"]
        self.assertIn("const LIMIT: usize = 262_144;", wire)
        self.assertIn("bytes.split_at_checked(8)?", wire)
        self.assertNotIn("_sequence_mut()", wire)

    def test_actual_41f_sources_and_custom_payload_wildcard(self):
        for path, digest in META["official_source_sha256"].items():
            self.assertEqual(render.sha((OFFICIAL / path).read_bytes()), digest)
        raw = (OFFICIAL / "crates/horizon-client/src/game/remote.rs").read_text()
        lines = raw.splitlines(keepends=True)
        start = next(i for i, line in enumerate(lines) if "fn handle_packet(" in line)
        end = next(i + 1 for i in range(start + 1, len(lines)) if lines[i] == "    }\n")
        method = "".join(lines[start:end])
        self.assertEqual(render.sha(method.encode()), "75fcb20347a46b79e47fd8ebac8610770c20f41a104bcfc9d3b993d5842ddf2e")
        self.assertLess(method.index("self.native_client_packet(&packet)"), method.index("match packet"))
        arms = re.findall(r"C::CustomPayload\(p\) if p.channel == ([^ ]+) =>", method)
        self.assertEqual(arms, ["horizon_protocol::mod_net::CHANNEL", "horizon_protocol::attachment_sync::CHANNEL", "horizon_protocol::menu_extra::CHANNEL", "horizon_client::lan::recipes::CONTENT_CHANNEL"])
        self.assertTrue(method.endswith("            _ => {}\n        }\n        Ok(())\n    }\n"))
        self.assertNotIn("tconstruct:materials_book", method)

    def test_declared_native_tests_are_unrun_and_source_only(self):
        self.assertEqual((render.HERE / "native-bridge.rs").read_text().count("#[test]"), 6)
        self.assertFalse(META["actual_rust_tests_run"])
        self.assertFalse(META["activation"])


if __name__ == "__main__": unittest.main()
