#[cfg(test)]
#[path = "../../../mixins/tconstruct/source-gravel-crafting.rs"]
mod gravel;

#[cfg(test)]
mod tests {
    use super::gravel;
    use std::path::PathBuf;
    struct Root(PathBuf);
    impl Root {
        fn new() -> Self {
            static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
            let root = std::env::temp_dir().join(format!("gravel-crafting-{}-{}", std::process::id(), NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)));
            std::fs::create_dir_all(root.join("config")).unwrap();
            Self(root)
        }
        fn set(&self, value: &str) { std::fs::write(self.0.join("config/tconstruct-common.toml"), value).unwrap(); }
        fn files(&self) -> Vec<(String, Vec<u8>)> { let mut files = Vec::new(); gravel::contribute(&self.0, &mut files); files }
    }
    impl Drop for Root { fn drop(&mut self) { let _ = std::fs::remove_dir_all(&self.0); } }

    #[test]
    fn default_and_explicit_true_make_exact_three_gravel_recipe() {
        let root = Root::new();
        for value in [None, Some("[recipes]\naddGravelToFlintRecipe = true\n")] {
            if let Some(value) = value { root.set(value); }
            let files = root.files();
            assert_eq!(files.len(), 2);
            assert_eq!(files[0].0, "recipe/tconstruct:common/flint");
            assert_eq!(files[1].0, gravel::ADVANCEMENT_PATH);
            let mut source: serde_json::Value = serde_json::from_str(include_str!("../../fixtures/flint_discovery/flint.json")).unwrap();
            source["criteria"]["has_the_recipe"]["conditions"] = serde_json::json!({"recipes":"tconstruct:common/flint"});
            assert_eq!(serde_json::from_slice::<serde_json::Value>(&files[1].1).unwrap(), source);
            assert_eq!(serde_json::from_slice::<serde_json::Value>(&files[0].1).unwrap(), serde_json::json!({
                "type":"minecraft:crafting_shapeless", "category":"misc",
                "ingredients":["minecraft:gravel","minecraft:gravel","minecraft:gravel"],
                "result":{"id":"minecraft:flint","count":1}}));
        }
    }

    #[test]
    fn false_unknown_malformed_and_unreadable_configs_fail_closed() {
        let root = Root::new();
        for value in ["[recipes]\naddGravelToFlintRecipe = false", "[recipes]\naddGravelToFlintRecipe = 'true'", "[recipes", "recipes = false"] {
            root.set(value);
            assert!(root.files().is_empty(), "{value}");
        }
        std::fs::remove_file(root.0.join("config/tconstruct-common.toml")).unwrap();
        std::fs::create_dir(root.0.join("config/tconstruct-common.toml")).unwrap();
        assert!(root.files().is_empty());
    }

    #[test]
    fn each_reload_and_world_uses_current_config_without_cached_recipe() {
        let a = Root::new();
        let b = Root::new();
        b.set("[recipes]\naddGravelToFlintRecipe = false");
        for enabled in [true, false, true, false] {
            a.set(&format!("[recipes]\naddGravelToFlintRecipe = {enabled}"));
            let files = a.files();
            assert_eq!(files.len(), if enabled { 2 } else { 0 });
            assert_eq!(files.iter().any(|(path, _)| path == gravel::PATH), enabled);
            assert_eq!(files.iter().any(|(path, _)| path == gravel::ADVANCEMENT_PATH), enabled);
            assert!(b.files().is_empty());
        }
    }

    #[test]
    fn preserves_other_contributions_and_existing_same_id() {
        let root = Root::new();
        let other = ("recipe/example:other".to_owned(), b"{}".to_vec());
        let own = (gravel::PATH.to_owned(), b"{\"override\":true}".to_vec());
        let mut files = vec![other.clone(), own.clone()];
        gravel::contribute(&root.0, &mut files);
        assert_eq!(files, vec![other.clone(), own]);
        let mut files = vec![other.clone()];
        gravel::contribute(&root.0, &mut files);
        assert_eq!(files[0], other);
        assert_eq!(files.len(), 3);
    }

    #[test]
    fn preserves_existing_discovery_and_does_not_duplicate_on_reentry() {
        let root = Root::new();
        let supplied = (gravel::ADVANCEMENT_PATH.to_owned(), b"{\"custom\":true}".to_vec());
        let mut files = vec![supplied.clone()];
        gravel::contribute(&root.0, &mut files);
        assert_eq!(files.len(), 2);
        assert_eq!(files[0], supplied);
        assert_eq!(files[1].0, gravel::PATH);
        let once = files.clone();
        gravel::contribute(&root.0, &mut files);
        assert_eq!(files, once);
        root.set("[recipes]\naddGravelToFlintRecipe = false");
        let mut disabled = vec![supplied.clone()];
        gravel::contribute(&root.0, &mut disabled);
        assert_eq!(disabled, vec![supplied]);
    }

}
