// Internal official-language adapter. No locale owner, dictionary or reload hook.
// Input sources are already admitted and ordered by the host/caller. This does
// not undo content-root flattening, pack filters or namespace ordering.
use horizon_client_core::{lang, text::Text};

/// Load a detached native value, without installing it or changing selection.
/// All parsing, en_us fallback, namespace/source order and deprecated-key rules
/// belong to the official loader. Missing inputs retain its native behavior.
pub fn load_ordered(sources: &[&dyn lang::ResourceSource], code: &str) -> lang::Language {
    lang::Language::load(sources, code)
}

/// Native current-language lookups, including scoped test-language behavior.
pub fn get(key: &str) -> String { lang::get(key) }
pub fn get_or(key: &str, fallback: &str) -> String { lang::get_or(key, fallback) }
pub fn has(key: &str) -> bool { lang::has(key) }
pub fn code() -> String { lang::code() }

/// Official plain formatting. No Java numeric-format conversion or key aliases.
pub fn tr(key: &str, args: &[&dyn std::fmt::Display]) -> String { lang::tr(key, args) }

/// Resolve now using the same native Text type. The result is a span snapshot,
/// not a live/reactive translation binding. Native component source is preserved;
/// callers must resolve again after locale changes to refresh the span snapshot.
pub fn text(key: &str, args: &[Text]) -> Text { Text::translate(key, args) }
pub fn text_or(key: &str, fallback: &str, args: &[Text]) -> Text {
    Text::translate_with_fallback(key, fallback, args)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    struct Source { namespaces: Vec<String>, files: BTreeMap<String, Vec<u8>> }
    impl lang::ResourceSource for Source {
        fn read(&self, path: &str) -> Option<Vec<u8>> { self.files.get(path).cloned() }
        fn namespaces(&self) -> Vec<String> { self.namespaces.clone() }
    }
    fn source(ns: &str, files: &[(&str, &str)]) -> Source {
        Source { namespaces: vec![ns.into()], files: files.iter().map(|(code,body)|
            (format!("assets/{ns}/lang/{code}.json"), body.as_bytes().to_vec())).collect() }
    }
    #[test]
    fn actual_loader_keeps_source_order_and_selected_over_english() {
        let low = source("meadow", &[("en_us", r#"{"k":"low","only":"English"}"#), ("ja_jp", r#"{"k":"日本語"}"#)]);
        let high = source("meadow", &[("en_us", r#"{"k":"high English"}"#), ("ja_jp", r#"{"only":"高"}"#)]);
        let en = load_ordered(&[&low,&high], "en_us");
        assert_eq!(en.get("k"), Some("high English"));
        let ja = load_ordered(&[&low,&high], "ja_jp");
        assert_eq!(ja.get("k"), Some("日本語"));
        assert_eq!(ja.get("only"), Some("高"));
    }
    #[test]
    fn namespace_order_is_not_replaced_by_global_source_priority() {
        let low = source("z_meadow", &[("en_us", r#"{"collision":"z low"}"#)]);
        let high = source("a_meadow", &[("en_us", r#"{"collision":"a high"}"#)]);
        assert_eq!(load_ordered(&[&low,&high], "en_us").get("collision"), Some("z low"));
    }
    #[test]
    fn current_fallbacks_and_missing_keys_use_native_scope() {
        let src=source("meadow", &[("en_us", r#"{"known":"English"}"#)]);
        let (_,missed)=lang::scoped(load_ordered(&[&src],"ja_jp"), || {
            assert_eq!(get("known"),"English"); assert!(has("known"));
            assert!(!has("missing")); assert_eq!(get("missing"),"missing");
            assert_eq!(get_or("missing","explicit"),"explicit");
            assert_eq!(code(),"ja_jp");
        });
        assert_eq!(missed,vec!["missing"]);
    }
    #[test]
    fn loading_does_not_set_selection_or_invent_regional_fallback() {
        let src=source("meadow", &[("en_us",r#"{"k":"English"}"#),("fr",r#"{"k":"French base"}"#)]);
        lang::scoped(load_ordered(&[&src],"en_us"),||{
            let detached=load_ordered(&[&src],"fr_ca");
            assert_eq!(detached.code,"fr_ca"); assert_eq!(detached.get("k"),Some("English"));
            assert_eq!(code(),"en_us");
        });
    }
    #[test]
    fn direct_native_lenient_and_partial_file_rules_are_unchanged() {
        let src=source("meadow", &[("en_us",r#"{"tail":"fallback"}"#),
            ("ja_jp", "{first='yes'; count=2; stop=[]; tail='not read'}")]);
        let loaded=load_ordered(&[&src],"ja_jp");
        assert_eq!(loaded.get("first"),Some("yes")); assert_eq!(loaded.get("count"),Some("2"));
        assert_eq!(loaded.get("tail"),Some("fallback")); assert_eq!(loaded.get("stop"),None);
    }
    #[test]
    fn missing_and_invalid_sources_do_not_clear_english() {
        let low=source("meadow", &[("en_us",r#"{"k":"English"}"#)]);
        let high=source("meadow", &[("ja_jp","[ not a language object ]")]);
        assert_eq!(load_ordered(&[&low,&high],"ja_jp").get("k"),Some("English"));
    }
    #[test]
    fn format_and_text_fallbacks_are_native_not_normalized() {
        let src=source("meadow", &[("en_us",r#"{"greet":"Hello %s %%","number":"%d units"}"#)]);
        lang::scoped(load_ordered(&[&src],"en_us"),||{
            assert_eq!(tr("greet",&[&"world"]),"Hello world %");
            assert_eq!(tr("number",&[&12]),"%d units");
            assert_eq!(text("greet",&[Text::literal("world")]).plain(),"Hello world %");
            assert_eq!(text_or("missing","Fallback %s",&[Text::literal("value")]).plain(),"Fallback value");
        });
    }
    #[test]
    fn custom_namespace_locale_is_not_native_menu_discovery() {
        let mut src=source("meadow", &[("qz_qz",r#"{"k":"custom"}"#)]);
        src.files.insert("pack.mcmeta".into(),br#"{"language":{"qz_qz":{"name":"Custom"}}}"#.to_vec());
        assert_eq!(load_ordered(&[&src],"qz_qz").get("k"),Some("custom"));
        assert!(!lang::available(&[&src]).iter().any(|v|v.code=="qz_qz"));
    }
}
