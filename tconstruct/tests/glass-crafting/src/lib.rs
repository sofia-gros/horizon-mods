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
            assert_eq!(files.len(), 4);
            for ((path, bytes), &(expected_path, expected_bytes)) in files.iter().zip(glass::RECIPES) {
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
            assert_eq!(a.files().len(), if enabled { 4 } else { 0 });
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
        assert_eq!(files.len(), 5);
        assert_eq!(files[0], other);
        assert_eq!(files[1], overridden);
        let once = files.clone();
        glass::contribute(&root.0, &mut files);
        assert_eq!(files, once);
    }
}
