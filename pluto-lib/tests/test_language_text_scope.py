"""Source-only facade guards; these do not compile or execute native Text."""
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / "mixins/pluto_native_creative/language-text.rs"


class LanguageTextScope(unittest.TestCase):
    def test_one_existing_adapter_call_per_resolution(self):
        source = SOURCE.read_text().split("#[cfg(test)]", 1)[0]
        body = source.split("pub fn resolve(self, args: &[Text]) -> Text {", 1)[1]
        self.assertEqual(body.strip(), """match self.fallback {
            Some(fallback) => crate::pluto_language::text_or(self.name, fallback, args),
            None => crate::pluto_language::text(self.name, args),
        }
    }
}""")

    def test_native_type_and_borrowed_configuration_only(self):
        source = SOURCE.read_text().split("#[cfg(test)]", 1)[0]
        for token in ["pub use crate::text::Text;", "name: &'a str", "fallback: Option<&'a str>", "'a: 'b", "pub const fn key", "pub const fn new"]:
            self.assertIn(token, source)
        for token in ["static ", "String", "Vec<", "Mutex", "RwLock", "OnceLock", "unsafe", "lang::set", "format_translation(", "Game<"]:
            self.assertNotIn(token, source)

    def test_actual_native_test_contract_is_explicit(self):
        source = SOURCE.read_text()
        self.assertIn("use crate::lang::{Language, ResourceSource, scoped};", source)
        self.assertEqual(source.count("#[test]"), 6)
        for name in [
            "retained_key_resolves_again_but_previous_text_is_a_snapshot",
            "missing_key_and_explicit_fallback_keep_native_components",
            "existing_selected_and_english_entries_win_over_fallback",
            "reordered_styled_arguments_use_native_text_without_flattening",
            "malformed_and_missing_placeholders_keep_native_fallback_behavior",
            "static_key_accepts_shorter_borrowed_fallback_without_registration",
        ]:
            self.assertIn("fn " + name + "()", source)
        example = (ROOT / "examples/language_text.rs").read_text()
        self.assertIn("use horizon_client::pluto_language::{Key, Text, key};", example)
        self.assertIn("COUNT.resolve(&[Text::literal(page_count.to_string())])", example)


if __name__ == "__main__":
    unittest.main()
