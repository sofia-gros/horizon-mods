#[cfg(test)]
#[path = "../../../mixins/tconstruct/source-slime-crafting.rs"]
mod slime;

#[cfg(test)]
mod tests {
    use super::slime;
    use std::path::PathBuf;
    struct Root(PathBuf);
    impl Root {
        fn new() -> Self {
            static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
            let root = std::env::temp_dir().join(format!("slime-crafting-{}-{}", std::process::id(), NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)));
            std::fs::create_dir_all(root.join("config")).unwrap();
            Self(root)
        }
        fn set(&self, value: &str) { std::fs::write(self.0.join("config/tconstruct-common.toml"), value).unwrap(); }
        fn files(&self) -> Vec<(String, Vec<u8>)> { let mut files = Vec::new(); slime::contribute(&self.0, &mut files); files }
    }
    impl Drop for Root { fn drop(&mut self) { let _ = std::fs::remove_dir_all(&self.0); } }

    #[test]
    fn source_default_and_true_supply_both_exact_recipes_and_discovery() {
        let root = Root::new();
        for value in [None, Some("[recipes]\nglassRecipeFix = true")] {
            if let Some(value) = value { root.set(value); }
            let files = root.files();
            assert_eq!(files.len(), 4);
            let lead: serde_json::Value = serde_json::from_slice(&files[0].1).unwrap();
            assert_eq!(files[0].0, "recipe/tconstruct:common/slime/lead");
            assert_eq!(lead["pattern"], serde_json::json!(["~~ ","~O ","  ~"]));
            assert_eq!(lead["key"], serde_json::json!({"O":"#forge:slimeballs","~":"minecraft:string"}));
            assert_eq!(lead["result"], serde_json::json!({"id":"minecraft:lead","count":2}));
            let piston: serde_json::Value = serde_json::from_slice(&files[2].1).unwrap();
            assert_eq!(files[2].0, "recipe/tconstruct:common/slime/sticky_piston");
            assert_eq!(piston["pattern"], serde_json::json!(["#","P"]));
            assert_eq!(piston["key"], serde_json::json!({"#":"#forge:slimeballs","P":"minecraft:piston"}));
            assert_eq!(piston["result"], serde_json::json!({"id":"minecraft:sticky_piston","count":1}));
        }
    }

    #[test]
    fn pinned_source_uses_glass_flag_not_slime_flag() {
        let root = Root::new();
        root.set("[recipes]\nglassRecipeFix = false\nslimeRecipeFix = true");
        assert!(root.files().is_empty());
        root.set("[recipes]\nglassRecipeFix = true\nslimeRecipeFix = false");
        assert_eq!(root.files().len(), 4);
    }

    #[test]
    fn false_unknown_malformed_and_read_errors_fail_closed() {
        let root = Root::new();
        for value in ["[recipes]\nglassRecipeFix = false", "[recipes]\nglassRecipeFix = 'true'", "[recipes", "recipes = false"] {
            root.set(value);
            assert!(root.files().is_empty(), "{value}");
        }
        std::fs::remove_file(root.0.join("config/tconstruct-common.toml")).unwrap();
        std::fs::create_dir(root.0.join("config/tconstruct-common.toml")).unwrap();
        assert!(root.files().is_empty());
    }

    #[test]
    fn repeated_loads_drop_disabled_recipes_and_keep_worlds_separate() {
        let a = Root::new();
        let b = Root::new();
        b.set("[recipes]\nglassRecipeFix = false");
        for enabled in [true, false, true, false] {
            a.set(&format!("[recipes]\nglassRecipeFix = {enabled}"));
            assert_eq!(a.files().len(), if enabled { 4 } else { 0 });
            assert!(b.files().is_empty());
        }
    }

    #[test]
    fn existing_same_id_and_other_contributions_are_preserved() {
        let root = Root::new();
        let other = ("recipe/example:other".to_owned(), b"{}".to_vec());
        let own = ("recipe/tconstruct:common/slime/lead".to_owned(), b"{\"override\":true}".to_vec());
        let mut files = vec![other.clone(), own.clone()];
        slime::contribute(&root.0, &mut files);
        assert_eq!(files.len(), 4);
        assert_eq!(files[0], other);
        assert_eq!(files[1], own);
        assert!(!files.iter().any(|(path, _)| path == "advancement/tconstruct:recipes/tools/common/slime/lead"));
        slime::contribute(&root.0, &mut files);
        assert_eq!(files.len(), 4);
    }

    #[test]
    fn source_discovery_uses_native_tag_and_recipe_event_predicates() {
        let root = Root::new();
        let files = root.files();
        let sources = [
            ("lead", "tools", include_bytes!("../../fixtures/configured_slime_discovery/lead.json").as_slice()),
            ("sticky_piston", "redstone", include_bytes!("../../fixtures/configured_slime_discovery/sticky_piston.json").as_slice()),
        ];
        assert_eq!(slime::ADVANCEMENTS.len(), sources.len());
        for (name, group, source) in sources {
            let recipe = format!("tconstruct:common/slime/{name}");
            let path = format!("advancement/tconstruct:recipes/{group}/common/slime/{name}");
            let mut expected: serde_json::Value = serde_json::from_slice(source).unwrap();
            expected["criteria"]["has_slime_ball"]["conditions"]["items"] = serde_json::json!([{"items":"#forge:slimeballs"}]);
            expected["criteria"]["has_the_recipe"]["conditions"] = serde_json::json!({"recipes": recipe});
            let native: serde_json::Value = serde_json::from_slice(&files.iter().find(|(id, _)| id == &path).unwrap().1).unwrap();
            assert_eq!(native, expected);
        }
    }

    #[test]
    fn earlier_advancement_survives_without_blocking_our_recipe() {
        let root = Root::new();
        let own = ("advancement/tconstruct:recipes/tools/common/slime/lead".to_owned(), b"{\"custom\":true}".to_vec());
        let mut files = vec![own.clone()];
        slime::contribute(&root.0, &mut files);
        assert_eq!(files.len(), 4);
        assert_eq!(files[0], own);
        assert!(files.iter().any(|(path, _)| path == "recipe/tconstruct:common/slime/lead"));
        let before = files.clone();
        slime::contribute(&root.0, &mut files);
        assert_eq!(files, before);
    }
}
