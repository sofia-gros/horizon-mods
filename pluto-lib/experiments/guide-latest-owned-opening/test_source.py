"""Pure source/guard tests. No Rust, Cargo, host application or UI execution."""
import copy
import importlib.util
import json
from pathlib import Path
import unittest

HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location("latest_guide", HERE / "generate.py")
m = importlib.util.module_from_spec(spec)
spec.loader.exec_module(m)


class LatestSource(unittest.TestCase):
    def test_frozen_dependencies_and_latest_pin(self):
        pins = m.config()
        self.assertEqual(pins["official_horizon"], m.PIN)
        self.assertEqual(pins["official_mixins"], "5a64af938ec86b00d53d16a408aabc0038fb3e57")
        self.assertEqual(len(pins["source_sha256"]), 21)

    def test_single_unchanged_helper_and_owned_fragments(self):
        pins = m.config()
        self.assertEqual(m.sha(m.combined_helper().encode()), pins["combined_helper_sha256"])
        self.assertEqual(pins["combined_helper_sha256"], "0bf154aa213fdcdb5aeecf0f079f137feaa44f4a25451fe8c51bfcf78a93b398")

    def test_frozen_known_delta_and_inverse_hashes(self):
        pins = m.config()
        result = json.loads((HERE / "generation.json").read_text())
        self.assertEqual(result["source_sha256"], pins["source_sha256"])
        self.assertEqual(result["restored_sha256"], pins["source_sha256"])
        self.assertEqual(len(result["rewrites"]), 25)
        self.assertEqual(len(result["changed_files"]), 13)
        self.assertEqual(m.sha((HERE / "prototype.patch").read_bytes()), result["patch_sha256"])

    def test_partial_receipt_never_claims_full_inventory_or_runtime(self):
        result = json.loads((HERE / "generation.json").read_text())
        self.assertIs(result["full_inventory_verified"], False)
        self.assertIs(result["latest_typing_executed"], False)
        self.assertIs(result["ui_or_lan_executed"], False)

    def fixture(self):
        source = {m.PREFIX + "game.rs": "impl Game<'_> {\n    fn f(&mut self) {\n        self.book.take();\n    }\n}\n"}
        book, transitions = m.inventory(source)
        return source, {"book_field_inventory": book, "attachment_inventory": transitions}

    def test_new_book_site_is_rejected_with_exact_path(self):
        source, pins = self.fixture()
        path = m.PREFIX + "game/new_screen.rs"
        source[path] = "fn f() { self.book.take(); }\n"
        with self.assertRaisesRegex(ValueError, "game/new_screen.rs"):
            m.check_inventory(source, pins)

    def test_new_actor_reset_is_rejected_with_exact_path(self):
        source, pins = self.fixture()
        path = m.PREFIX + "game/new_reset.rs"
        source[path] = "self.player = None;\n"
        with self.assertRaisesRegex(ValueError, "game/new_reset.rs"):
            m.check_inventory(source, pins)

    def test_existing_site_line_or_text_drift_is_rejected(self):
        source, pins = self.fixture()
        source[m.PREFIX + "game.rs"] = "\n" + source[m.PREFIX + "game.rs"]
        with self.assertRaisesRegex(ValueError, "game.rs"):
            m.check_inventory(source, pins)

    def test_unrelated_source_does_not_invent_a_lifecycle_site(self):
        source, pins = self.fixture()
        source[m.PREFIX + "game/unrelated.rs"] = "fn unrelated() {}\n"
        m.check_inventory(source, pins)

    def test_known_input_set_cannot_be_extended_or_trimmed(self):
        with self.assertRaisesRegex(ValueError, "file set"):
            m.build_known_inputs({}, m.config())

    def test_independent_hunk_reader_rejects_changed_context(self):
        old = m.legacy()
        before = {"demo.rs": "one\nold\nthree\n"}
        after = {"demo.rs": "one\nnew\nthree\n"}
        patch = old.patch_text(before, after)
        self.assertEqual(old.apply_in_memory(patch, before), after)
        self.assertEqual(old.apply_in_memory(old.patch_text(after, before), after), before)
        damaged = copy.deepcopy(before)
        damaged["demo.rs"] = "different\nold\nthree\n"
        with self.assertRaisesRegex(ValueError, "context mismatch"):
            old.apply_in_memory(patch, damaged)


    def fixture_transitions(self):
        return {path: "".join(before for before, _ in rules)
                for path, rules in m.FIXTURE_TRANSITIONS.items()}

    def test_reviewed_fixture_assignments_are_all_transformed_and_invertible(self):
        source = self.fixture_transitions()
        result, changes = m.extend_fixture_transitions(source)
        self.assertEqual(len(changes), 3)
        self.assertEqual(sum(v.count("invalidate_attachment()") for v in result.values()), 3)
        self.assertNotEqual(result, source)
        for path, rules in m.FIXTURE_TRANSITIONS.items():
            restored = result[path]
            for before, after in reversed(rules): restored = restored.replace(after, before)
            self.assertEqual(restored, source[path])

    def test_fallible_rhs_finishes_before_invalidation_or_assignment(self):
        result, _ = m.extend_fixture_transitions(self.fixture_transitions())
        source = result[m.PREFIX + "game/boat_rider_fixture.rs"]
        self.assertLess(source.index("PlayerMotion::new("), source.index("?;"))
        self.assertLess(source.index("?;"), source.index("self.book.invalidate_attachment();"))
        self.assertLess(source.index("self.book.invalidate_attachment();"), source.index("self.player = Some(player);"))
        self.assertEqual(source.count("PlayerMotion::new("), 1)

    def test_fixture_missing_duplicate_or_changed_assignment_stops(self):
        for path, rules in m.FIXTURE_TRANSITIONS.items():
            for before, _ in rules:
                for replacement in ["", before + before, before.replace("self.", "other.")]:
                    source = self.fixture_transitions()
                    source[path] = source[path].replace(before, replacement)
                    with self.assertRaisesRegex(ValueError, "fixture transition drift"):
                        m.extend_fixture_transitions(source)

    def test_reviewed_fixture_inventory_does_not_exclude_another_new_site(self):
        source = self.fixture_transitions()
        book, transitions = m.inventory(source)
        self.assertEqual(sum(len(x["sites"]) for x in transitions), 3)
        pins = {"book_field_inventory": book, "attachment_inventory": transitions}
        m.check_inventory(source, pins)
        source[m.PREFIX + "game/boat_rider_fixture.rs"] += "self.remote = None;\n"
        with self.assertRaisesRegex(ValueError, "boat_rider_fixture.rs"):
            m.check_inventory(source, pins)


if __name__ == "__main__":
    unittest.main()
