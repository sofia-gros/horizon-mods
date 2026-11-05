#[cfg(test)]
#[path = "../../../mixins/tconstruct/source-glass-crafting.rs"]
mod glass;

#[cfg(test)]
mod tests {
    use super::glass;
    use std::path::PathBuf;

    struct Root(PathBuf);
    impl Root {
        fn new() -> Self {
            static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
            let path = std::env::temp_dir().join(format!("glass-crafting-{}-{}", std::process::id(), NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)));
            std::fs::create_dir_all(path.join("config")).unwrap();
            Self(path)
        }
        fn set(&self, value: &str) { std::fs::write(self.0.join("config/tconstruct-common.toml"), value).unwrap(); }
        fn files(&self) -> Vec<(String, Vec<u8>)> {
            let mut files = Vec::new();
            glass::contribute(&self.0, &mut files);
            files
        }
    }
    impl Drop for Root { fn drop(&mut self) { let _ = std::fs::remove_dir_all(&self.0); } }

    #[test]
    fn default_and_explicit_true_contribute_four_native_shaped_recipes() {
        let root = Root::new();
        for config in [None, Some("[recipes]\nglassRecipeFix = true\n")] {
            if let Some(value) = config { root.set(value); }
            let files = root.files();
            assert_eq!(files.len(), 8);
            for ((path, bytes), &(expected_path, expected_bytes)) in files.iter().filter(|(path, _)| path.starts_with("recipe/")).zip(glass::RECIPES) {
                assert_eq!(path, expected_path);
                assert_eq!(bytes, expected_bytes);
                let recipe: serde_json::Value = serde_json::from_slice(bytes).unwrap();
                assert_eq!(recipe["type"], "minecraft:crafting_shaped");
                assert!(recipe.get("conditions").is_none());
                assert!(recipe["key"].as_object().unwrap().values().any(|v| v == "#forge:glass/colorless"));
                assert!(recipe["result"]["id"].as_str().unwrap().starts_with("minecraft:"));
            }
        }
    }

    #[test]
    fn false_invalid_and_unreadable_configs_fail_closed() {
        let root = Root::new();
        for config in ["[recipes]\nglassRecipeFix = false", "[recipes]\nglassRecipeFix = 'true'", "[recipes", "recipes = false"] {
            root.set(config);
            assert!(root.files().is_empty(), "{config}");
        }
        std::fs::remove_file(root.0.join("config/tconstruct-common.toml")).unwrap();
        std::fs::create_dir(root.0.join("config/tconstruct-common.toml")).unwrap();
        assert!(root.files().is_empty());
    }

    #[test]
    fn reload_and_world_changes_do_not_reuse_another_worlds_setting() {
        let a = Root::new();
        let b = Root::new();
        b.set("[recipes]\nglassRecipeFix = false");
        for enabled in [true, false, true, false] {
            a.set(&format!("[recipes]\nglassRecipeFix = {enabled}"));
            assert_eq!(a.files().len(), if enabled { 8 } else { 0 });
            assert!(b.files().is_empty());
        }
    }

    #[test]
    fn preserves_other_recipes_and_partial_same_id_overrides_without_duplicates() {
        let root = Root::new();
        let other = ("recipe/example:other".to_owned(), b"{}".to_vec());
        let overridden = (glass::RECIPES[1].0.to_owned(), b"{\"override\":true}".to_vec());
        let mut files = vec![other.clone(), overridden.clone()];
        glass::contribute(&root.0, &mut files);
        assert_eq!(files.len(), 8);
        assert!(!files.iter().any(|(path, _)| path == glass::ADVANCEMENTS[1].1));
        assert_eq!(files[0], other);
        assert_eq!(files[1], overridden);
        let once = files.clone();
        glass::contribute(&root.0, &mut files);
        assert_eq!(files, once);
    }

    #[test]
    fn source_discovery_is_paired_with_each_recipe_and_adapts_native_predicates() {
        let root = Root::new();
        let files = root.files();
        let fixtures = [
            ("glass_bottle", include_str!("../../fixtures/configured_glass_discovery/glass_bottle.json")),
            ("beacon", include_str!("../../fixtures/configured_glass_discovery/beacon.json")),
            ("daylight_detector", include_str!("../../fixtures/configured_glass_discovery/daylight_detector.json")),
            ("end_crystal", include_str!("../../fixtures/configured_glass_discovery/end_crystal.json")),
        ];
        assert_eq!(glass::ADVANCEMENTS.len(), fixtures.len());
        for (name, raw) in fixtures {
            let id = format!("tconstruct:common/glass/vanilla/{name}");
            let recipe_path = format!("recipe/{id}");
            let &(_, advancement_path, _) = glass::ADVANCEMENTS.iter().find(|row| row.0 == recipe_path).unwrap();
            assert!(files.iter().any(|(path, _)| path == &recipe_path));
            let bytes = &files.iter().find(|(path, _)| path == advancement_path).unwrap().1;
            let mut expected: serde_json::Value = serde_json::from_str(raw).unwrap();
            expected["criteria"]["has_the_recipe"]["conditions"] = serde_json::json!({"recipes":id});
            if name == "glass_bottle" {
                expected["criteria"]["has_glass"]["conditions"] = serde_json::json!({"items":[{"items":"#forge:glass/colorless"}]});
            }
            assert_eq!(serde_json::from_slice::<serde_json::Value>(bytes).unwrap(), expected);
        }
    }

    #[test]
    fn existing_advancement_is_preserved_without_blocking_its_recipe() {
        let root = Root::new();
        let supplied = (glass::ADVANCEMENTS[1].1.to_owned(), b"{\"custom\":true}".to_vec());
        let mut files = vec![supplied.clone()];
        glass::contribute(&root.0, &mut files);
        assert_eq!(files.len(), 8);
        assert_eq!(files[0], supplied);
        assert!(files.iter().any(|(path, _)| path == glass::ADVANCEMENTS[1].0));
        let once = files.clone();
        glass::contribute(&root.0, &mut files);
        assert_eq!(files, once);
        root.set("[recipes]\nglassRecipeFix = false");
        let mut disabled = vec![supplied.clone()];
        glass::contribute(&root.0, &mut disabled);
        assert_eq!(disabled, vec![supplied]);
    }

}
