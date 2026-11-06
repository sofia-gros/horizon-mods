//! Bounded native-name proof. The Game receiver, rendering and networking are not instantiated.
pub use horizon_client_core::{lang, text};
mod mod_content { pub use horizon_items::content_namespaces::namespaces; }
mod entity_nbt { pub use horizon_client_core::component::name_text; }
#[path = "native_names.rs"] mod native_names;
#[path = "native_styled_name.rs"] mod native_styled_name;
#[path = "language.rs"] pub mod pluto_language;
#[path = "language-text.rs"] pub mod pluto_language_text;

extern crate self as horizon_client;
// Public-path type check against the actual modules in this bounded crate, not installed Game.
#[path = "language_text_example.rs"] mod author_example;

#[cfg(test)]
mod item_display_tests {
    use super::*;
    use horizon_items::{item_nbt, items::ItemStack};
    use horizon_world::nbt::Tag;
    use std::{collections::BTreeMap, fs, path::PathBuf, sync::atomic::{AtomicU64, Ordering}};
    use serde_json::{Value, json};

    static NEXT: AtomicU64 = AtomicU64::new(0);
    struct Root(PathBuf);
    impl Root {
        fn names() -> (Self, native_names::Names) {
            let path = std::env::temp_dir().join(format!("pluto-item-language-{}-{}", std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed)));
            // Exclusive creation means Drop only removes a directory this test created.
            fs::create_dir(&path).unwrap();
            let root = Self(path);
            fs::create_dir_all(root.0.join("assets/example/lang")).unwrap();
            fs::create_dir_all(root.0.join("assets/minecraft/lang")).unwrap();
            fs::write(root.0.join("horizon-mod-content.json"), br#"["example"]"#).unwrap();
            fs::write(root.0.join("assets/example/lang/en_us.json"), br#"{"item.example.widget":"English widget","block.example.widget":"English block","block.example.block_only":"English block only","item.example.english_only":"English only","item.example.path/with_underscore":"Path key","gui.example.english":"English label"}"#).unwrap();
            fs::write(root.0.join("assets/minecraft/lang/en_us.json"), br#"{"item.minecraft.stone":"English stone"}"#).unwrap();
            let names = native_names::Names::load(&root.0);
            (root, names)
        }
    }
    impl Drop for Root { fn drop(&mut self) { fs::remove_dir_all(&self.0).unwrap(); } }
    struct Source(BTreeMap<String, Vec<u8>>);
    impl lang::ResourceSource for Source {
        fn read(&self, path: &str) -> Option<Vec<u8>> { self.0.get(path).cloned() }
        fn namespaces(&self) -> Vec<String> { vec!["example".into(), "minecraft".into()] }
    }
    fn source() -> Source {
        Source([
            ("en_us", json!({"item.example.named":"Named %s", "item.example.widget":"Resource widget", "gui.example.value":"Value"})),
            ("ja_jp", json!({"item.example.named":"名前 %s", "item.example.widget":"日本語道具", "block.example.widget":"日本語ブロック", "block.example.block_only":"日本語ブロックのみ", "gui.example.value":"値"})),
        ].into_iter().map(|(code, value)| (format!("assets/example/lang/{code}.json"), serde_json::to_vec(&value).unwrap())).collect())
    }
    fn scoped<R>(code: &str, f: impl FnOnce() -> R) -> R {
        lang::scoped(pluto_language::load_ordered(&[&source()], code), f).0
    }
    fn tag(value: &Value) -> Tag { horizon_client_core::scoreboard::json_to_tag(value) }
    fn named_stack(component: Tag) -> ItemStack {
        item_nbt::item_from_nbt(&Tag::compound([
            ("id", Tag::String("example:widget".into())),
            ("count", Tag::Int(1)),
            ("components", Tag::compound([("minecraft:item_name", component)])),
        ])).unwrap()
    }

    #[test]
    fn standard_keys_use_current_then_english_fallback() {
        let (_root, names) = Root::names();
        scoped("ja_jp", || {
            assert_eq!(names.display_item("example:widget"), "日本語道具");
            assert_eq!(names.display_item("example:block_only"), "日本語ブロックのみ");
            assert_eq!(names.display_item("example:english_only"), "English only");
            assert_eq!(names.display_get("gui.example.value").as_deref(), Some("値"));
            assert_eq!(names.display_get("gui.example.english").as_deref(), Some("English label"));
            assert_eq!(names.display_get("gui.example.absent"), None);
        });
    }
    #[test]
    fn standard_paths_preserve_slashes_and_implicit_namespace() {
        let (_root, names) = Root::names();
        scoped("ja_jp", || {
            assert_eq!(names.display_item("example:path/with_underscore"), "Path key");
            assert_eq!(names.display_item("stone"), "English stone");
            assert_eq!(names.display_item("example:unknown/path_with_spaces"), "unknown/path with spaces");
        });
    }
    #[test]
    fn item_name_is_resolved_again_without_mutating_the_component() {
        let component = tag(&json!({"translate":"item.example.named","with":["A"]}));
        let stack = named_stack(component.clone());
        assert_eq!(scoped("en_us", || stack.item_name()).as_deref(), Some("Named A"));
        assert_eq!(scoped("ja_jp", || stack.item_name()).as_deref(), Some("名前 A"));
        assert_eq!(stack.data().unwrap().other("minecraft:item_name"), Some(&component));
    }
    #[test]
    fn custom_name_wins_and_preserves_explicit_italic() {
        let mut stack = named_stack(tag(&json!({"translate":"item.example.named","with":["A"]})));
        let custom = tag(&json!({"text":"Owner name","color":"#123456","italic":false}));
        stack.data_mut().custom_name = Some(custom.clone());
        scoped("ja_jp", || {
            assert_eq!(stack.custom_name().as_deref(), Some("Owner name"));
            let got = native_styled_name::styled_name(&stack, || panic!("custom name must not request fallback"));
            assert_eq!(got.plain(), "Owner name");
            assert_eq!(got.spans[0].style.color, Some(0x123456));
            assert!(!got.spans[0].style.is_italic());
        });
        assert_eq!(stack.data().unwrap().custom_name.as_ref(), Some(&custom));
    }
    #[test]
    fn native_codec_keeps_translate_arguments_and_style() {
        // The public Key is a thin wrapper over the same native Text. Keep its
        // component, not resolved plain(), in the stack's item_name patch.
        let component = scoped("en_us", || pluto_language::key("item.example.named")
            .resolve(&[text::Text::literal("A").with_color(0x123456)]).component());
        assert_eq!(component["translate"], "item.example.named");
        assert!(component.get("with").is_some());
        let original = tag(&component);
        let stack = named_stack(original.clone());
        let saved = item_nbt::item_to_nbt(&stack);
        assert_eq!(saved.get("components").unwrap().get("minecraft:item_name"), Some(&original));
        let loaded = item_nbt::item_from_nbt(&saved).unwrap();
        assert_eq!(item_nbt::item_to_nbt(&loaded), saved);
        assert_eq!(scoped("en_us", || loaded.item_name()).as_deref(), Some("Named A"));
        scoped("ja_jp", || {
            let got = native_styled_name::styled_name(&loaded, || panic!("item_name must not request fallback"));
            assert_eq!(got.plain(), "名前 A");
            assert!(got.spans.iter().any(|s| s.text == "A" && s.style.color == Some(0x123456)));
        });
        assert_eq!(loaded.data().unwrap().other("minecraft:item_name"), Some(&original));
    }
    #[test]
    fn missing_key_and_explicit_fallback_are_native() {
        scoped("ja_jp", || {
            let missing = pluto_language::key("item.example.absent").resolve(&[]);
            assert_eq!(named_stack(tag(&missing.component())).item_name().as_deref(), Some("item.example.absent"));
            let fallback = pluto_language::key("item.example.absent").with_fallback("Fallback %s").resolve(&[text::Text::literal("A")]);
            let stack = named_stack(tag(&fallback.component()));
            assert_eq!(stack.item_name().as_deref(), Some("Fallback A"));
            assert_eq!(fallback.component()["fallback"], "Fallback %s");
        });
    }
    #[test]
    fn styled_name_defers_unadorned_stack_to_current_default() {
        let (_root, names) = Root::names();
        let stack = ItemStack::new("example:widget", 1);
        scoped("ja_jp", || {
            let mut calls = 0;
            let got = native_styled_name::styled_name(&stack, || { calls += 1; names.display_item(&stack.item) });
            assert_eq!(calls, 1);
            assert_eq!(got.plain(), "日本語道具");
        });
    }
    #[test]
    fn construction_does_not_fabricate_an_item_name_patch() {
        let plain = ItemStack::new("example:widget", 1);
        assert!(plain.data().is_none());
        assert_eq!(plain.item_name(), None);
        assert!(item_nbt::item_to_nbt(&plain).get("components").is_none());
        let explicit = named_stack(tag(&json!({"translate":"item.example.named","with":["A"]})));
        assert!(explicit.data().unwrap().other("minecraft:item_name").is_some());
        // This does not claim that a content declaration's default component
        // is automatically materialized by every other creation path.
    }
}
