"""Read-only source regressions, not Cargo/Mixins/host execution."""
import importlib.util
import json
import os
from pathlib import Path
import tempfile
import unittest

HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location("book_composition", HERE / "stage_composition.py")
composition = importlib.util.module_from_spec(spec)
spec.loader.exec_module(composition)


class StageCompositionTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.stage = Path(os.environ["PLUTO_REVIEWED_V15_STAGE"])
        cls.seal = Path(os.environ["PLUTO_REVIEWED_V15_SEAL"])
        cls.official = Path(os.environ["PLUTO_OFFICIAL_SOURCE"])
        cls.patch, cls.receipt = composition.compose(cls.stage, cls.seal, cls.official)

    def test_stored_delta_matches_exact_sealed_source(self):
        self.assertEqual(self.patch, (HERE / "v15-composition.patch").read_text())
        self.assertEqual(self.receipt, json.loads((HERE / "v15-composition.json").read_text()))
        self.assertEqual(self.receipt["stage_file_count"], 5047)
        self.assertEqual(len(self.receipt["changed_files"]), 11)

    def test_pristine_rules_reject_reprinted_books_instead_of_silent_adaptation(self):
        g = composition.generator()
        inputs = {p: (self.stage / p).read_text() for p in self.receipt["input_sha256"]}
        with self.assertRaisesRegex(ValueError, "rewrite count drift"):
            g.transform(inputs)

    def test_only_the_reviewed_multiline_match_rule_changes(self):
        g = composition.generator()
        before = g.RULES
        after = composition.stage_rules(before)
        self.assertEqual({k: v for k, v in before.items() if k != "game/books.rs"},
                         {k: v for k, v in after.items() if k != "game/books.rs"})
        self.assertIn(("matches!(self.book,", "matches!(self.book.as_ref(),", 4), after["game/books.rs"])
        self.assertIn(("matches!(self.book,", "matches!(self.book.as_ref(),", 1), before["game/books.rs"])

    def test_complete_appended_modules_stay_identical(self):
        tails = self.receipt["unchanged_appended_helper_tails"]
        self.assertIn("crates/horizon-client/src/game/books.rs", tails)
        self.assertIn("crates/horizon-client/src/game.rs", tails)
        self.assertNotIn("pluto_current_book", "\n".join(line for line in self.patch.splitlines() if line.startswith(("+", "-"))))

    def test_wrong_seal_rejected_before_stage_read(self):
        with tempfile.TemporaryDirectory(prefix="book-seal-negative-") as temporary:
            wrong = Path(temporary) / "seal.json"
            wrong.write_text("{}")
            with self.assertRaisesRegex(ValueError, "not the reviewed full v15 seal"):
                composition.sealed_files(Path(temporary) / "nonexistent", wrong)

    def test_no_manifest_or_guard_or_transport_change_is_in_delta(self):
        self.assertTrue(all(p.startswith("crates/horizon-client/src/") for p in self.receipt["changed_files"]))
        self.assertNotIn("Cargo.toml", self.patch)
        self.assertNotIn("observe(packet)", self.patch)
        self.assertEqual(self.receipt["pristine_patch_sha256"], "fe77790980cf115f674ec593ea4efce9f9a8c11bcbcefd3f21af0781f0be0628")


if __name__ == "__main__":
    unittest.main()
