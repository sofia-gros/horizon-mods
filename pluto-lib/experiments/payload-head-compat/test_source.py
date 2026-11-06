"""Pure source/finite-branch checks, not actual Rust codecs or Game execution."""
import copy
import importlib.util
import json
import os
import re
from pathlib import Path
import unittest

HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location("payload_heads", HERE / "compose.py")
C = importlib.util.module_from_spec(spec); spec.loader.exec_module(C)
M = json.loads((HERE / "inputs.json").read_text(encoding="utf-8"))
O = json.loads((HERE / "oracles.json").read_text(encoding="utf-8"))
INPUT = Path(os.environ["PAYLOAD_HEAD_INPUT_ROOT"])
GUIDE = Path(os.environ["PAYLOAD_GUIDE_ROOT"])
PLAN = Path(os.environ["PAYLOAD_PLAN"])
OFFICIAL = Path(os.environ["PAYLOAD_OFFICIAL_ROOT"])
CHANNELS = Path(os.environ["PAYLOAD_CHANNEL_ROOT"])


def originals():
    return {p: (INPUT / p).read_text(encoding="utf-8") for p in M["original_sha256"]}


def guides():
    return {p: (GUIDE / p).read_text(encoding="utf-8") for p in M["guide_postimage_sha256"]}


class SourceTests(unittest.TestCase):
    def test_old_oracle_and_codec_inputs_exact(self):
        for p, raw in O.items():
            self.assertEqual(C.sha(raw.encode()), M["original_sha256"][p])
            self.assertEqual(raw, (INPUT / p).read_text(encoding="utf-8"))
        for p, digest in M["unchanged_codec_sha256"].items():
            self.assertEqual(C.sha((INPUT / p).read_bytes()), digest)

    def test_two_helpers_are_exact_reversible_control_flow_edits(self):
        for p, old in O.items():
            actual = (HERE / Path(p).name).read_text(encoding="utf-8")
            self.assertEqual(C.helper(old), actual)
            self.assertNotIn("ControlFlow", actual)
            self.assertEqual(actual.count("::decode(&message.data)"), 1)
            self.assertEqual(actual.count(" = None;"), 1)

    def test_composition_has_nine_exact_outputs(self):
        out = C.render(originals(), guides())
        expected = json.loads((HERE / "outputs.json").read_text())
        self.assertEqual(set(out), set(expected["outputs"]))
        self.assertEqual(len(out), 9)
        for p, text in out.items():
            self.assertEqual(C.sha(text.encode()), expected["outputs"][p]["sha256"])

    def test_manifest_only_changes_two_flags_over_guide(self):
        original = guides()[C.TC + "horizon-mixin.toml"]
        revised = C.manifest(original)
        for n in C.IDS:
            old = C.table(original, "injection", n + "-wire-receive")
            new = C.table(revised, "injection", n + "-wire-receive")
            self.assertEqual(new.replace("cancellable = false", "cancellable = true"), old)
            revised = revised.replace(new, old)
        self.assertEqual(revised, original)

    def test_idempotent_reapplication_is_rejected(self):
        for old in O.values():
            with self.assertRaises(ValueError): C.helper(C.helper(old))
        with self.assertRaises(ValueError): C.manifest(C.manifest(guides()[C.TC + "horizon-mixin.toml"]))

    def test_unknown_original_or_guide_bytes_rejected(self):
        for group in ["original", "guide"]:
            a = originals(); b = guides()
            changed = a if group == "original" else b
            path = next(iter(changed)); changed[path] += "\n"
            with self.assertRaises(ValueError): C.render(a, b)
        b = guides(); b["unexpected.rs"] = ""
        with self.assertRaises(ValueError): C.render(originals(), b)

    def test_missing_or_duplicate_manifest_table_rejected(self):
        original = guides()[C.TC + "horizon-mixin.toml"]
        block = C.table(original, "injection", "broad-wire-receive")
        for bad in [original.replace(block, ""), original + "\n" + block]:
            with self.assertRaises(ValueError): C.manifest(bad)

    def test_cloud_move_commutes_and_chest_deferral_remains(self):
        original = originals()[C.TC + "horizon-mixin.toml"]
        move = M["cloud_move"]
        self.assertEqual(original.count(move["new"]), 1)
        old = original.replace(move["new"], move["old"])
        self.assertEqual(C.sha(old.encode()), move["before_manifest_sha256"])
        self.assertEqual(C.manifest(old).replace(move["old"], move["new"]), C.manifest(original))
        out = C.render(originals(), guides())[C.TC + "horizon-mixin.toml"]
        self.assertIn(move["new"], out)
        self.assertNotIn('id = "tinkers-chest-pick-color"', out)
        self.assertIn('id = "materials-book-text"', out)
        self.assertIn('id = "materials-book-index"', out)

    def test_finite_decoded_fact_oracle_preserves_writes(self):
        # Decode values are symbolic facts, not a Python reimplementation of
        # native wire/NBT. Exact decode expressions and bytes are pinned above.
        def old(state, packet, channel):
            kind, received, decoded = packet
            if kind == "tags": return None, False, 1
            if kind != "custom" or received != channel: return state, False, 0
            return decoded, True, 1
        def new(state, packet, channel):
            kind, received, decoded = packet
            if kind == "tags": return None, False, 1
            if kind != "custom" or received != channel: return state, False, 0
            return decoded, False, 1
        count = 0
        for channel in ["tconstruct:overwield_broad", "tconstruct:haste_chest"]:
            for state in [None, (), ("old",), ("other", 2)]:
                for packet in [("tags", None, None), ("other", None, None), ("custom", "unknown:channel", ("ignored",)), ("custom", channel, None), ("custom", channel, ()), ("custom", channel, ("fresh",))]:
                    a = old(state, packet, channel); b = new(state, packet, channel)
                    self.assertEqual((a[0], a[2]), (b[0], b[2]))
                    self.assertFalse(b[1])
                    self.assertEqual(a[1], packet[0] == "custom" and packet[1] == channel)
                    count += 1
        self.assertEqual(count, 48)

    def test_original_complete_plan_has_exact_three_cancelling_heads(self):
        self.assertEqual(C.sha(PLAN.read_bytes()), M["actual_plan_sha256"])
        plan = json.loads(PLAN.read_text())
        hooks = [h for h in plan["injections"] if h["target"] == M["handle_target"]]
        cancelled = [h for h in hooks if h.get("cancellable")]
        self.assertEqual({h["injection_id"] for h in cancelled}, set(M["cancelled_ids"]))
        for h in cancelled:
            self.assertEqual(h["kind"], "HEAD")
            self.assertEqual(h["node"], [44, 68])
            self.assertEqual(h["function_fingerprint"], M["handle_fingerprint"])

    def test_official_remote_source_and_native_custom_guards_unchanged(self):
        path = "crates/horizon-client/src/game/remote.rs"
        raw = (OFFICIAL / path).read_bytes()
        self.assertEqual(C.sha(raw), M["official_source_sha256"][path])
        source = raw.decode()
        body = source.split("fn handle_packet(", 1)[1]
        self.assertLess(body.index("self.native_client_packet(&packet);"), body.index("match packet"))
        for channel in ["tconstruct:overwield_broad", "tconstruct:haste_chest", "tconstruct:materials_book"]:
            self.assertNotIn(channel, body)
        for marker in ["mod_net::CHANNEL", "attachment_sync::CHANNEL", "menu_extra::CHANNEL", "recipes::CONTENT_CHANNEL"]:
            self.assertIn(marker, body)
        found = set()
        for p, entry in M["native_channels"].items():
            raw = (CHANNELS / p).read_bytes()
            self.assertEqual(C.sha(raw), entry["sha256"])
            matches = re.findall(r'pub const (?:CONTENT_CHANNEL|CHANNEL): &str = "([^"]+)";', raw.decode())
            self.assertEqual(matches, [entry["channel"]])
            found.update(matches)
        self.assertEqual(len(found), 4)
        self.assertTrue(found.isdisjoint({"tconstruct:overwield_broad", "tconstruct:haste_chest", "tconstruct:materials_book"}))


if __name__ == "__main__": unittest.main()
