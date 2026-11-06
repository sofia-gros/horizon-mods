// Uncompiled baseline fixture. No production merge/locale/consumer changes.
// Uses actual public official mod_content::build and PathBuf ResourceSource.
#[cfg(test)]
mod tests {
    use horizon_client_core::lang::Language;
    use horizon_client_lib::mod_content::{self, ContentMod};
    use horizon_mods::{Content, Package};
    use std::{collections::BTreeMap, fs, path::{Path, PathBuf}, sync::atomic::{AtomicU64, Ordering}};

    static NEXT: AtomicU64 = AtomicU64::new(0);
    const EN: &str = "assets/meadow/lang/en_us.json";
    const JA: &str = "assets/meadow/lang/ja_jp.json";
    const LOWER: &str = r#"{"gui.meadow.shared":"pack","gui.meadow.lower":"lower only"}"#;
    const HIGHER: &str = r#"{"gui.meadow.shared":"mod","gui.meadow.higher":"higher only"}"#;

    struct Scratch(PathBuf);
    impl Scratch {
        fn new() -> Self {
            for _ in 0..128 {
                let p = std::env::temp_dir().join(format!("pluto-language-overlay-{}-{}", std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed)));
                match fs::create_dir(&p) {
                    Ok(()) => return Self(p),
                    Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                    Err(e) => panic!("owned scratch: {e}"),
                }
            }
            panic!("scratch names exhausted")
        }
        fn output(&self) -> PathBuf { self.0.join("owned-output") }
        fn base(&self) -> PathBuf {
            let p = self.0.join("owned-base");
            fs::create_dir(&p).unwrap();
            // Only the exact table schema consumed by this asset-only build.
            // These are not complete playable game assets or simulated native types.
            put(&p, "facts.json", br#"{"states":[["minecraft:air"]],"defaults":{"minecraft:air":0}}"#);
            put(&p, "gameplay-facts.json", br#"{"states":[{}]}"#);
            put(&p, "items.json", b"{}");
            p
        }
    }
    impl Drop for Scratch {
        fn drop(&mut self) {
            let result = fs::remove_dir_all(&self.0);
            if !std::thread::panicking() { result.expect("remove only owned scratch"); }
        }
    }
    fn put(root: &Path, relative: &str, bytes: &[u8]) {
        let path = root.join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, bytes).unwrap();
    }
    fn source_bytes(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
        fn visit(root: &Path, at: &Path, out: &mut BTreeMap<PathBuf, Vec<u8>>) {
            for entry in fs::read_dir(at).unwrap() {
                let entry = entry.unwrap(); let kind = entry.file_type().unwrap();
                assert!(!kind.is_symlink(), "fixture-owned base has no symlinks");
                if kind.is_dir() { visit(root, &entry.path(), out); }
                else { assert!(kind.is_file()); out.insert(entry.path().strip_prefix(root).unwrap().to_owned(), fs::read(entry.path()).unwrap()); }
            }
        }
        let mut out = BTreeMap::new(); visit(root, root, &mut out); out
    }
    fn content(scratch: &Scratch, id: &str, extra_namespace: Option<&str>, files: &[(&str, &[u8])]) -> (ContentMod, PathBuf, Vec<u8>) {
        let mut manifest = format!("id = \"{id}\"\nname = \"Overlay fixture\"\nversion = \"0.0.0\"\napi = \"0.2\"\ncapabilities = [\"registries\"]\n");
        if let Some(ns) = extra_namespace { manifest.push_str(&format!("namespaces = [\"{ns}\"]\n")); }
        let bytes = Package::write_files(&manifest, None, files).expect("actual package writer");
        let path = scratch.0.join(format!("{id}.hmod"));
        fs::write(&path, &bytes).unwrap();
        let package = Package::read(&path).expect("actual package reader");
        let content = Content::read(&path, &package.manifest).expect("actual content admission");
        (ContentMod { id: package.manifest.id, content }, path, bytes)
    }

    #[test]
    fn mod_replaces_whole_locale_and_never_mutates_base_or_package() {
        let s = Scratch::new(); let base = s.base(); let out = s.output();
        put(&base, EN, LOWER.as_bytes()); let before = source_bytes(&base);
        let (m, package, bytes) = content(&s, "meadow", None, &[(EN, HIGHER.as_bytes())]);
        assert!(mod_content::build(&base, &[m], &out).unwrap().is_empty());
        assert_eq!(fs::read(out.join(EN)).unwrap(), HIGHER.as_bytes());
        let language = Language::load(&[&out], "en_us");
        assert_eq!(language.get("gui.meadow.shared"), Some("mod"));
        assert_eq!(language.get("gui.meadow.higher"), Some("higher only"));
        assert_eq!(language.get("gui.meadow.lower"), None, "baseline is whole-file replacement, not key retention");
        assert_eq!(source_bytes(&base), before); assert_eq!(fs::read(package).unwrap(), bytes);
    }

    #[test]
    fn untouched_selected_locale_survives_and_exact_missing_code_uses_output_english() {
        let s = Scratch::new(); let base = s.base(); let out = s.output();
        put(&base, EN, LOWER.as_bytes());
        let ja = r#"{"gui.meadow.shared":"パック","gui.meadow.selected":"日本語"}"#;
        put(&base, JA, ja.as_bytes()); let before = source_bytes(&base);
        let (m, _, _) = content(&s, "meadow", None, &[(EN, HIGHER.as_bytes())]);
        mod_content::build(&base, &[m], &out).unwrap();
        assert_eq!(fs::read(out.join(JA)).unwrap(), ja.as_bytes());
        let selected = Language::load(&[&out], "ja_jp");
        assert_eq!(selected.get("gui.meadow.shared"), Some("パック"));
        assert_eq!(selected.get("gui.meadow.higher"), Some("higher only"));
        assert_eq!(Language::load(&[&out], "ja").get("gui.meadow.shared"), Some("mod"));
        assert_eq!(source_bytes(&base), before);
    }

    #[test]
    fn distinct_namespaces_keep_native_language_order_not_mod_input_order() {
        let s = Scratch::new(); let base = s.base(); let out = s.output();
        let (woods, _, _) = content(&s, "woods", None, &[("assets/woods/lang/en_us.json", br#"{"gui.shared":"woods"}"#)]);
        let (meadow, _, _) = content(&s, "meadow", None, &[(EN, br#"{"gui.shared":"meadow"}"#)]);
        let mods = [woods, meadow];
        mod_content::build(&base, &mods, &out).unwrap();
        assert_eq!(Language::load(&[&out], "en_us").get("gui.shared"), Some("woods"));
        let [woods, meadow] = mods;
        mod_content::build(&base, &[meadow, woods], &out).unwrap();
        assert_eq!(Language::load(&[&out], "en_us").get("gui.shared"), Some("woods"));
    }

    #[test]
    fn overlapping_owned_namespace_rejects_before_owned_output_recreation() {
        let s = Scratch::new(); let base = s.base(); let out = s.output();
        put(&out, "sentinel", b"old owned output"); let before = source_bytes(&base);
        let (a, _, _) = content(&s, "first", Some("shared"), &[("assets/shared/lang/en_us.json", b"{}")]);
        let (b, _, _) = content(&s, "second", Some("shared"), &[("assets/shared/lang/en_us.json", b"{}")]);
        let error = mod_content::build(&base, &[a, b], &out).unwrap_err().to_string();
        assert!(error.contains("shared"), "{error}");
        assert_eq!(fs::read(out.join("sentinel")).unwrap(), b"old owned output");
        assert_eq!(source_bytes(&base), before);
    }

    #[test]
    fn malformed_mod_locale_is_copied_verbatim_and_native_loader_falls_back() {
        let s = Scratch::new(); let base = s.base(); let out = s.output();
        put(&base, EN, LOWER.as_bytes()); put(&base, JA, br#"{"gui.meadow.shared":"old selected"}"#);
        let before = source_bytes(&base);
        let (m, _, _) = content(&s, "meadow", None, &[(JA, b"not json")]);
        mod_content::build(&base, &[m], &out).unwrap();
        assert_eq!(fs::read(out.join(JA)).unwrap(), b"not json");
        assert_eq!(Language::load(&[&out], "ja_jp").get("gui.meadow.shared"), Some("pack"));
        assert_eq!(source_bytes(&base), before);
    }

    #[test]
    fn missing_admitted_package_fails_after_owned_output_recreation_without_rollback_claim() {
        let s = Scratch::new(); let base = s.base(); let out = s.output();
        put(&base, EN, LOWER.as_bytes()); put(&out, "sentinel", b"old owned output");
        let before = source_bytes(&base);
        let (m, package, _) = content(&s, "meadow", None, &[(EN, HIGHER.as_bytes())]);
        fs::remove_file(package).unwrap();
        assert!(mod_content::build(&base, &[m], &out).is_err());
        assert!(!out.join("sentinel").exists(), "native build is not an atomic old-output preservation promise");
        assert_eq!(source_bytes(&base), before);
    }

    #[test]
    fn malformed_required_base_table_rejects_before_owned_output_recreation() {
        let s = Scratch::new(); let base = s.base(); let out = s.output();
        put(&base, "facts.json", br#"{"defaults":{}}"#); put(&out, "sentinel", b"preserved");
        let before = source_bytes(&base);
        let (m, _, _) = content(&s, "meadow", None, &[(EN, HIGHER.as_bytes())]);
        let error = mod_content::build(&base, &[m], &out).unwrap_err().to_string();
        assert!(error.contains("facts.json: no states"), "{error}");
        assert_eq!(fs::read(out.join("sentinel")).unwrap(), b"preserved");
        assert_eq!(source_bytes(&base), before);
    }

    #[test]
    fn rebuilding_only_owned_output_removes_previous_mod_file_and_restores_base() {
        let s = Scratch::new(); let base = s.base(); let out = s.output();
        put(&base, EN, LOWER.as_bytes()); let before = source_bytes(&base);
        let extra = "assets/meadow/lang/ja_jp.json";
        let (first, _, _) = content(&s, "meadow", None, &[(EN, HIGHER.as_bytes()), (extra, b"{}")]);
        mod_content::build(&base, &[first], &out).unwrap();
        assert!(out.join(extra).is_file());
        let (second, _, _) = content(&s, "woods", None, &[("assets/woods/lang/en_us.json", b"{}")]);
        mod_content::build(&base, &[second], &out).unwrap();
        assert!(!out.join(extra).exists());
        assert_eq!(fs::read(out.join(EN)).unwrap(), LOWER.as_bytes());
        assert_eq!(source_bytes(&base), before);
    }
}
