//! Translations: text in the player's language (not in Fabric API; Minecraft: `Language`, `Text.translatable`).
//!
//! A mod registers its text per language under keys such as `fluid.example.molten_copper`; PlutoLib looks a key up in the language the
//! game is set to, falls back to `en_us`, and last to the key itself so a missing translation shows up instead of breaking anything. The
//! game's own language files (`assets/<mod>/lang/<language>.json`, resource packs included) are consulted too. When the player changes
//! the language option, [`LanguageEvents::CHANGED`] fires.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use super::base::Event;

/// The language everything falls back to.
pub const FALLBACK_LANGUAGE: &str = "en_us";

static TRANSLATIONS: Mutex<Option<HashMap<String, HashMap<String, String>>>> = Mutex::new(None);
static OVERRIDE: Mutex<Option<String>> = Mutex::new(None);
static HOST: OnceLock<HostLanguage> = OnceLock::new();
static SEEN: Mutex<Option<String>> = Mutex::new(None);

/// What the game says about the language (set once by the game-facing half).
pub(crate) struct HostLanguage {
    /// The code of the language the game is set to.
    pub code: fn() -> String,
    /// The game's own translation of a key, if its language files have one.
    pub get: fn(&str) -> Option<String>,
}

pub(crate) fn set_host(host: HostLanguage) {
    let _ = HOST.set(host);
}

/// Registers text per language.
pub struct Translations;

impl Translations {
    /// Adds `entries` (key and text) for `language` (`en_us`, `ja_jp`, ...). A key registered again replaces the earlier text.
    pub fn register(language: &str, entries: impl IntoIterator<Item = (String, String)>) {
        if let Ok(mut all) = TRANSLATIONS.lock() {
            all.get_or_insert_with(HashMap::new).entry(language.to_owned()).or_default().extend(entries);
        }
    }

    /// Adds the entries of a language file (a JSON object of keys and texts, the format of `lang/en_us.json`); returns how many were read.
    pub fn register_json(language: &str, json: &str) -> Result<usize, serde_json::Error> {
        let entries: HashMap<String, String> = serde_json::from_str(json)?;
        let count = entries.len();
        Self::register(language, entries);
        Ok(count)
    }

    /// The languages something was registered for, sorted.
    pub fn languages() -> Vec<String> {
        let mut found: Vec<String> = TRANSLATIONS.lock().ok().and_then(|m| m.as_ref().map(|m| m.keys().cloned().collect())).unwrap_or_default();
        found.sort();
        found
    }
}

fn registered(language: &str, key: &str) -> Option<String> {
    TRANSLATIONS.lock().ok()?.as_ref()?.get(language)?.get(key).cloned()
}

/// Looking text up in the language the game is set to.
pub struct I18n;

impl I18n {
    /// The code of the language in use: the one set with [`I18n::use_language`], else the game's, else `en_us`.
    pub fn language() -> String {
        if let Some(code) = OVERRIDE.lock().ok().and_then(|o| o.clone()) {
            return code;
        }
        HOST.get().map_or_else(|| FALLBACK_LANGUAGE.to_owned(), |h| (h.code)())
    }

    /// Uses `language` instead of the game's until called with `None`. For tests, and for code that writes for one player (a server
    /// message in that player's language).
    pub fn use_language(language: Option<&str>) {
        if let Ok(mut o) = OVERRIDE.lock() {
            *o = language.map(str::to_owned);
        }
    }

    /// The text for `key` in the language in use: what a mod registered, then what the game's language files say, then the `en_us` text,
    /// then the key itself.
    pub fn translate(key: &str) -> String {
        let language = Self::language();
        if let Some(text) = registered(&language, key) {
            return text;
        }
        if let Some(text) = HOST.get().filter(|_| OVERRIDE.lock().ok().is_none_or(|o| o.is_none())).and_then(|h| (h.get)(key)) {
            return text;
        }
        registered(FALLBACK_LANGUAGE, key).unwrap_or_else(|| key.to_owned())
    }

    /// The text for `key` in `language` (what a mod registered for it, else `en_us`, else the key), whatever the game is set to.
    pub fn translate_in(language: &str, key: &str) -> String {
        registered(language, key).or_else(|| registered(FALLBACK_LANGUAGE, key)).unwrap_or_else(|| key.to_owned())
    }

    /// `translate` with arguments: `%s` takes the next argument and `%1$s`, `%2$s` the numbered ones (Minecraft's format); `%%` is a percent.
    pub fn translate_args(key: &str, args: &[&str]) -> String {
        format_args_into(&Self::translate(key), args)
    }

    /// Whether any text is registered or known to the game for `key` in the language in use or `en_us`.
    pub fn has(key: &str) -> bool {
        let language = Self::language();
        registered(&language, key).is_some()
            || registered(FALLBACK_LANGUAGE, key).is_some()
            || HOST.get().is_some_and(|h| (h.get)(key).is_some())
    }
}

fn format_args_into(template: &str, args: &[&str]) -> String {
    let mut out = String::new();
    let mut next = 0;
    let mut chars = template.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '%' {
            out.push(c);
            continue;
        }
        match chars.peek().copied() {
            Some('%') => {
                chars.next();
                out.push('%');
            }
            Some('s') => {
                chars.next();
                out.push_str(args.get(next).copied().unwrap_or(""));
                next += 1;
            }
            Some(d) if d.is_ascii_digit() => {
                let mut digits = String::new();
                let mut probe = chars.clone();
                while let Some(&n) = probe.peek() {
                    if n.is_ascii_digit() {
                        digits.push(n);
                        probe.next();
                    } else {
                        break;
                    }
                }
                if probe.next() == Some('$') && probe.next() == Some('s') {
                    chars = probe;
                    let index: usize = digits.parse().unwrap_or(0);
                    out.push_str(args.get(index.wrapping_sub(1)).copied().unwrap_or(""));
                } else {
                    out.push('%');
                }
            }
            _ => out.push('%'),
        }
    }
    out
}

/// A change of the language option.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LanguageChange {
    /// The code of the language before.
    pub previous: String,
    /// The code of the language now.
    pub current: String,
}

static CHANGED: Event<LanguageChange, ()> = Event::void();

/// Events about the language.
pub struct LanguageEvents;

impl LanguageEvents {
    /// The player changed the language option (checked every game tick). Redraw or rebuild anything you cached in the old language.
    pub const CHANGED: &'static Event<LanguageChange, ()> = &CHANGED;
}

/// Compares the language in use with the last one seen and fires `CHANGED` on a change; called every tick by the game-facing half.
pub(crate) fn poll_language() {
    let current = I18n::language();
    let previous = {
        let Ok(mut seen) = SEEN.lock() else { return };
        match seen.replace(current.clone()) {
            Some(previous) if previous != current => previous,
            _ => return,
        }
    };
    CHANGED.invoke(&LanguageChange { previous, current });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex as TestLock;

    static ONE_AT_A_TIME: TestLock<()> = TestLock::new(());

    #[test]
    fn text_comes_from_the_language_in_use_falls_back_to_english_then_to_the_key_and_takes_arguments() {
        let _only = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
        Translations::register("en_us", [("t.greet".to_owned(), "Hello, %s!".to_owned()), ("t.only_en".to_owned(), "English only".to_owned())]);
        Translations::register("ja_jp", [("t.greet".to_owned(), "こんにちは、%s！".to_owned())]);
        I18n::use_language(Some("ja_jp"));
        assert_eq!(I18n::language(), "ja_jp");
        assert_eq!(I18n::translate_args("t.greet", &["Alex"]), "こんにちは、Alex！");
        assert_eq!(I18n::translate("t.only_en"), "English only", "a missing translation falls back to en_us");
        assert_eq!(I18n::translate("t.nowhere"), "t.nowhere", "and last to the key");
        assert_eq!(I18n::translate_in("en_us", "t.greet"), "Hello, %s!");
        assert_eq!(I18n::translate_in("fr_fr", "t.greet"), "Hello, %s!");
        assert!(I18n::has("t.greet") && !I18n::has("t.nowhere"));
        I18n::use_language(None);
    }

    #[test]
    fn arguments_follow_minecrafts_format_and_a_language_file_is_read_as_json() {
        let _only = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
        assert_eq!(format_args_into("%2$s then %1$s, 50%% of %s", &["a", "b"]), "b then a, 50% of a");
        assert_eq!(format_args_into("%s %s", &["only"]), "only ");
        assert_eq!(Translations::register_json("en_us", r#"{"t.json.a": "A", "t.json.b": "B"}"#).unwrap(), 2);
        assert!(Translations::register_json("en_us", "not json").is_err());
        assert!(Translations::languages().contains(&"en_us".to_owned()));
    }

    #[test]
    fn changing_the_language_fires_the_event_once_with_the_old_and_new_code() {
        let _only = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
        use std::sync::Arc;
        let seen = Arc::new(Mutex::new(Vec::new()));
        let sink = seen.clone();
        LanguageEvents::CHANGED.register(move |change| sink.lock().unwrap().push((change.previous.clone(), change.current.clone())));
        I18n::use_language(Some("en_us"));
        poll_language();
        seen.lock().unwrap().clear();
        I18n::use_language(Some("ja_jp"));
        poll_language();
        poll_language();
        assert_eq!(*seen.lock().unwrap(), [("en_us".to_owned(), "ja_jp".to_owned())]);
        I18n::use_language(None);
    }
}
