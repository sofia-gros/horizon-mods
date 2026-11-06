"""Exact inverse source checks, not Rust/native/UI execution."""
import hashlib
import json
from pathlib import Path
import unittest

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
MOD = ROOT / "tconstruct/mixins/tconstruct"


class IdentityScope(unittest.TestCase):
    def test_every_changed_existing_file_restores_exact_previous_bytes(self):
        oracle = json.loads((HERE / "identity-refactor-oracle.json").read_text())
        self.assertEqual(len(oracle["records"]), 5)
        for record in oracle["records"]:
            with self.subTest(path=record["path"]):
                current = (ROOT / record["path"]).read_text()
                self.assertEqual(current.count(record["new"]), 1)
                restored = current.replace(record["new"], record["old"])
                self.assertEqual(hashlib.sha256(restored.encode()).hexdigest(), record["before_sha256"])

    def test_caller_root_checks_and_different_unknown_outcomes_stay_explicit(self):
        draw = (MOD / "source-materials-book-draw.rs").read_text()
        wire = (MOD / "source-materials-book-wire.rs").read_text()
        self.assertLess(draw.index('!= Some("tconstruct:materials-book-root")'), draw.index("legacy_draw_sprites(title)"))
        self.assertLess(wire.index("if !root(pages) { return None; }"), wire.index("legacy_wire_id(title)"))
        source = (MOD / "source-materials-book-identity.rs").read_text().split("#[cfg(test)]", 1)[0]
        self.assertIn("legacy_title(title).map(BookId::wire_name)", source)
        self.assertIn("legacy_title(title).unwrap_or(BookId::Materials).sprites()", source)
        self.assertIn("tail.starts_with('\\n')", source)

    def test_consumer_mapping_adds_no_runtime_authority_or_translation(self):
        source = (MOD / "source-materials-book-identity.rs").read_text().split("#[cfg(test)]", 1)[0]
        for token in ["Game<", "Text", "Atomic", "Mutex", "RwLock", "unsafe", "lang::", "send(", "open_book", "String", "Vec<"]:
            self.assertNotIn(token, source)
        self.assertEqual(source.count('"tconstruct:'), 6)
        self.assertEqual(source.count('"assets/tconstruct/book/'), 6)

    def test_test_scope_is_five_pure_cases_not_installed_consumer_proof(self):
        source = (MOD / "source-materials-book-identity.rs").read_text()
        self.assertEqual(source.count("#[test]"), 5)
        self.assertIn("fn old_draw(title: &str)", source)
        self.assertIn("fn old_wire(title: &str)", source)
        fixture = (HERE / "src/lib.rs").read_text()
        self.assertIn('source-materials-book-identity.rs', fixture)
        self.assertIn("pub struct Text { pub spans: Vec<Span> }", fixture)


if __name__ == "__main__":
    unittest.main()
