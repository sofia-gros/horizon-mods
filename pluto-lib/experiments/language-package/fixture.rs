// SOURCE-ONLY fixture candidate. No production adapter or resource pipeline.
// Uses the actual official Package, Content and Language implementations.
#[cfg(test)]
mod tests {
    use horizon_client_core::lang::{self, Language, ResourceSource};
    use horizon_mods::{Content, Package};
    use std::{fs, path::PathBuf, sync::atomic::{AtomicU64, Ordering}};

    const MANIFEST: &str = r#"id = "meadow"
name = "Language fixture"
version = "0.0.0"
api = "0.2"
capabilities = ["registries"]
"#;
    const EN: &str = r#"{"gui.meadow.title":"Field Guide","gui.meadow.only_english":"English","gui.meadow.format":"Count: %s"}"#;
    const JA: &str = r#"{"gui.meadow.title":"野原の手引き","gui.meadow.format":"数: %s"}"#;
    const EN_PATH: &str = "assets/meadow/lang/en_us.json";
    const JA_PATH: &str = "assets/meadow/lang/ja_jp.json";
    static NEXT: AtomicU64 = AtomicU64::new(0);

    // Only remove a directory this fixture successfully created exclusively.
    struct Scratch(PathBuf);
    impl Scratch {
        fn new() -> Self {
            for _ in 0..128 {
                let name = format!("pluto-language-package-{}-{}", std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed));
                let path = std::env::temp_dir().join(name);
                match fs::create_dir(&path) {
                    Ok(()) => return Self(path),
                    Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                    Err(e) => panic!("fixture scratch: {e}"),
                }
            }
            panic!("fixture scratch names exhausted")
        }
    }
    impl Drop for Scratch {
        fn drop(&mut self) {
            let result = fs::remove_dir_all(&self.0);
            if !std::thread::panicking() { result.expect("remove owned fixture scratch"); }
        }
    }

    fn write(scratch: &Scratch, manifest: &str, wasm: Option<&[u8]>, files: &[(&str, &[u8])]) -> PathBuf {
        let bytes = Package::write_files(manifest, wasm, files).expect("actual package writer");
        let path = scratch.0.join("language.hmod");
        fs::write(&path, bytes).expect("write own fixture package");
        path
    }
    fn admit(scratch: &Scratch, files: &[(&str, &[u8])]) -> Content {
        let path = write(scratch, MANIFEST, None, files);
        let package = Package::read(&path).expect("actual package reader");
        assert!(package.wasm.is_none());
        assert_eq!(package.manifest.id, "meadow");
        Content::read(&path, &package.manifest).expect("actual content admission")
    }

    // Test-only byte view: no key parsing, merge, format, locale state or fallback.
    // Missing unlisted files are None; an admitted file that cannot be read FAILS.
    // This is not the production Game::content_root/overlay adapter.
    struct Admitted<'a>(&'a Content);
    impl ResourceSource for Admitted<'_> {
        fn read(&self, path: &str) -> Option<Vec<u8>> {
            if self.0.files.binary_search_by(|name| name.as_str().cmp(path)).is_err() {
                return None;
            }
            Some(self.0.file(path).expect("admitted resource must remain readable"))
        }
        fn namespaces(&self) -> Vec<String> { self.0.namespaces.clone() }
    }

    #[test]
    fn hmod_round_trip_preserves_owned_locale_bytes_and_actual_lookup() {
        let scratch = Scratch::new();
        let content = admit(&scratch, &[(EN_PATH, EN.as_bytes()), (JA_PATH, JA.as_bytes())]);
        assert_eq!(content.files, vec![EN_PATH, JA_PATH]);
        assert_eq!(content.namespaces, vec!["meadow"]);
        assert_eq!(content.file(EN_PATH).unwrap(), EN.as_bytes());
        assert_eq!(content.file(JA_PATH).unwrap(), JA.as_bytes());
        let source = Admitted(&content);
        let english = Language::load(&[&source], "en_us");
        let japanese = Language::load(&[&source], "ja_jp");
        assert_eq!(english.get("gui.meadow.title"), Some("Field Guide"));
        assert_eq!(japanese.get("gui.meadow.title"), Some("野原の手引き"));
        assert_eq!(japanese.get("gui.meadow.only_english"), Some("English"));
        assert_eq!(japanese.get("gui.meadow.format"), Some("数: %s"));
        assert_eq!(japanese.get("gui.meadow.absent"), None);
    }

    #[test]
    fn packaged_missing_exact_locale_uses_english_not_regional_guess() {
        let scratch = Scratch::new();
        let content = admit(&scratch, &[(EN_PATH, EN.as_bytes()), (JA_PATH, JA.as_bytes())]);
        let source = Admitted(&content);
        let language = Language::load(&[&source], "ja");
        assert_eq!(language.code, "ja");
        assert_eq!(language.get("gui.meadow.title"), Some("Field Guide"));
    }

    #[test]
    fn content_admission_rejects_unowned_locale_namespace() {
        let scratch = Scratch::new();
        let path = write(&scratch, MANIFEST, None, &[("assets/other/lang/en_us.json", EN.as_bytes())]);
        let package = Package::read(&path).expect("manifest admission alone succeeds");
        let error = Content::read(&path, &package.manifest).unwrap_err();
        assert!(error.contains("own namespaces"), "{error}");
    }

    #[test]
    fn content_admission_requires_registries_even_for_only_language_files() {
        let scratch = Scratch::new();
        let manifest = MANIFEST.replace("[\"registries\"]", "[]");
        // Dummy bytes only satisfy Package's presence rule; no WASM is executed.
        let path = write(&scratch, &manifest, Some(b"\0asm"), &[(EN_PATH, EN.as_bytes())]);
        let package = Package::read(&path).expect("manifest with present code bytes");
        let error = Content::read(&path, &package.manifest).unwrap_err();
        assert!(error.contains("not capability registries"), "{error}");
    }

    #[test]
    fn package_toml_asset_is_not_runtime_language_support() {
        let scratch = Scratch::new();
        let content = admit(&scratch, &[(EN_PATH, EN.as_bytes()),
            ("assets/meadow/lang/ja_jp.toml", "\"gui.meadow.title\" = \"野原\"".as_bytes())]);
        assert_eq!(content.files.len(), 2);
        let source = Admitted(&content);
        assert_eq!(Language::load(&[&source], "ja_jp").get("gui.meadow.title"), Some("Field Guide"));
    }

    #[test]
    fn package_content_admission_does_not_validate_language_json_schema() {
        let scratch = Scratch::new();
        let content = admit(&scratch, &[(EN_PATH, EN.as_bytes()), (JA_PATH, b"not json")]);
        assert_eq!(content.file(JA_PATH).unwrap(), b"not json");
        let source = Admitted(&content);
        assert_eq!(Language::load(&[&source], "ja_jp").get("gui.meadow.title"), Some("Field Guide"));
    }

    #[test]
    fn custom_owned_locale_does_not_prove_native_locale_menu_discovery() {
        let scratch = Scratch::new();
        let content = admit(&scratch, &[(EN_PATH, EN.as_bytes()), (JA_PATH, JA.as_bytes())]);
        let source = Admitted(&content);
        assert!(!lang::available(&[&source]).iter().any(|entry| entry.code == "ja_jp"));
        assert_eq!(Language::load(&[&source], "ja_jp").get("gui.meadow.title"), Some("野原の手引き"));
    }

    #[test]
    fn admitted_file_loss_is_failure_not_missing_resource_success() {
        let scratch = Scratch::new();
        let content = admit(&scratch, &[(EN_PATH, EN.as_bytes())]);
        fs::remove_file(scratch.0.join("language.hmod")).unwrap();
        assert!(content.file(EN_PATH).is_err());
        let source = Admitted(&content);
        assert!(std::panic::catch_unwind(|| Language::load(&[&source], "en_us")).is_err());
    }
}
