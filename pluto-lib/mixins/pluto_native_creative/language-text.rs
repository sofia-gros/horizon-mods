// Thin author-facing translation keys. The native language service owns all
// lookup and formatting. A returned Text is a snapshot, not a live UI binding.
pub use crate::text::Text;

/// Borrowed translation settings. Construction performs no lookup or allocation.
///
/// Keep the key and resolve it again at the consumer's admitted display/rebuild
/// boundary when fresh text is needed. This does not subscribe to reloads, own
/// an open screen, choose a guest locale or authorize a network reply.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Key<'a> {
    name: &'a str,
    fallback: Option<&'a str>,
}

/// Create a key using native missing-key behavior (the key itself).
pub const fn key(name: &str) -> Key<'_> {
    Key::new(name)
}

impl<'a> Key<'a> {
    pub const fn new(name: &'a str) -> Self {
        Self { name, fallback: None }
    }

    /// Set the native fallback template, used only when the key is missing.
    /// It may borrow a value with a shorter lifetime than the original key.
    pub const fn with_fallback<'b>(self, fallback: &'b str) -> Key<'b>
    where
        'a: 'b,
    {
        Key { name: self.name, fallback: Some(fallback) }
    }

    /// Resolve once through the current official language and Text formatter.
    ///
    /// Arguments are already-resolved Text values. Their styles/component
    /// behavior is native; this does not recursively re-resolve old arguments.
    /// Native keys, placeholders and fallback behavior are not normalized.
    /// This is trusted in-process authoring, not a bounded untrusted codec.
    pub fn resolve(self, args: &[Text]) -> Text {
        match self.fallback {
            Some(fallback) => crate::pluto_language::text_or(self.name, fallback, args),
            None => crate::pluto_language::text(self.name, args),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lang::{Language, ResourceSource, scoped};

    struct Source;
    impl ResourceSource for Source {
        fn read(&self, path: &str) -> Option<Vec<u8>> {
            match path {
                "assets/example/lang/en_us.json" => Some(
                    br#"{"gui.example.title":"Guide","gui.example.args":"%2$s / %1$s / %2$s","gui.example.only_english":"English"}"#.to_vec(),
                ),
                "assets/example/lang/fr_fr.json" => Some(
                    br#"{"gui.example.title":"Manuel"}"#.to_vec(),
                ),
                _ => None,
            }
        }
        fn namespaces(&self) -> Vec<String> {
            vec!["example".into()]
        }
    }

    fn in_language<R>(code: &str, f: impl FnOnce() -> R) -> R {
        scoped(Language::load(&[&Source], code), f).0
    }

    #[test]
    fn retained_key_resolves_again_but_previous_text_is_a_snapshot() {
        const TITLE: Key<'static> = key("gui.example.title");
        let old = in_language("en_us", || TITLE.resolve(&[]));
        let new = in_language("fr_fr", || TITLE.resolve(&[]));
        assert_eq!(old.plain(), "Guide");
        assert_eq!(new.plain(), "Manuel");
        assert_eq!(old.translation_key(), Some("gui.example.title"));
        assert_eq!(new.translation_key(), old.translation_key());
    }

    #[test]
    fn missing_key_and_explicit_fallback_keep_native_components() {
        in_language("fr_fr", || {
            let missing = key("gui.example.absent").resolve(&[]);
            assert_eq!(missing.plain(), "gui.example.absent");
            assert_eq!(missing.component(), Text::translate("gui.example.absent", &[]).component());
            let fallback = key("gui.example.absent").with_fallback("Fallback %s");
            let args = [Text::literal("value")];
            let resolved = fallback.resolve(&args);
            assert_eq!(resolved.plain(), "Fallback value");
            assert_eq!(resolved.component(), Text::translate_with_fallback("gui.example.absent", "Fallback %s", &args).component());
        });
    }

    #[test]
    fn existing_selected_and_english_entries_win_over_fallback() {
        in_language("fr_fr", || {
            assert_eq!(key("gui.example.title").with_fallback("Wrong").resolve(&[]).plain(), "Manuel");
            assert_eq!(key("gui.example.only_english").with_fallback("Wrong").resolve(&[]).plain(), "English");
        });
    }

    #[test]
    fn reordered_styled_arguments_use_native_text_without_flattening() {
        in_language("en_us", || {
            let args = [Text::literal("first").with_color(0x123456), Text::literal("second").bold()];
            let got = key("gui.example.args").resolve(&args);
            let expected = Text::translate("gui.example.args", &args);
            assert_eq!(got.plain(), "second / first / second");
            assert_eq!(got.spans, expected.spans);
            assert_eq!(got.component(), expected.component());
        });
    }

    #[test]
    fn malformed_and_missing_placeholders_keep_native_fallback_behavior() {
        in_language("en_us", || {
            for template in ["unsupported %d", "missing %2$s", "percent %%", "trailing %"] {
                let args = [Text::literal("one")];
                let got = key("gui.example.absent").with_fallback(template).resolve(&args);
                let expected = Text::translate_with_fallback("gui.example.absent", template, &args);
                assert_eq!(got.spans, expected.spans);
                assert_eq!(got.component(), expected.component());
            }
        });
    }

    #[test]
    fn static_key_accepts_shorter_borrowed_fallback_without_registration() {
        const ABSENT: Key<'static> = Key::new("gui.example.absent");
        let fallback = String::from("Borrowed");
        let configured = ABSENT.with_fallback(&fallback);
        in_language("en_us", || assert_eq!(configured.resolve(&[]).plain(), "Borrowed"));
        assert_eq!(ABSENT, key("gui.example.absent"));
    }
}
