// Source-only, uncompiled fixture. Native pack and Mod owners remain unchanged.
#[cfg(test)]
mod tests {
    use horizon_client_core::lang::Language;
    use horizon_client_lib::mod_content::{self, ContentMod};
    use horizon_client_render::packs::{self, Pack};
    use horizon_mods::{Content, Package};
    use std::{collections::BTreeMap, fs, io::{Cursor, Write}, path::{Path, PathBuf}, sync::atomic::{AtomicU64, Ordering}};

    static NEXT: AtomicU64 = AtomicU64::new(0);
    const EN: &str = "assets/meadow/lang/en_us.json";
    const JA: &str = "assets/meadow/lang/ja_jp.json";
    const VANILLA: &[u8] = br#"{"gui.meadow.shared":"vanilla","gui.meadow.vanilla":"vanilla only"}"#;
    const MOD: &[u8] = br#"{"gui.meadow.shared":"mod","gui.meadow.mod":"mod only"}"#;

    struct Scratch(PathBuf);
    impl Scratch {
        fn new() -> Self {
            for _ in 0..128 {
                let path = std::env::temp_dir().join(format!("pluto-language-pack-chain-{}-{}", std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed)));
                match fs::create_dir(&path) {
                    Ok(()) => return Self(path),
                    Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                    Err(e) => panic!("owned scratch: {e}"),
                }
            }
            panic!("scratch names exhausted")
        }
        fn base(&self) -> PathBuf {
            let base = self.0.join("base");
            put(&base, "facts.json", br#"{"states":[["minecraft:air"]],"defaults":{"minecraft:air":0}}"#);
            put(&base, "gameplay-facts.json", br#"{"states":[{}]}"#);
            put(&base, "items.json", b"{}");
            put(&base, EN, VANILLA);
            base
        }
        fn folder(&self, name: &str, filter: bool, files: &[(&str, &[u8])]) {
            let root = self.0.join("packs").join(name);
            put(&root, "pack.mcmeta", metadata(filter).as_bytes());
            for (path, bytes) in files { put(&root, path, bytes); }
        }
        fn selected(&self, names: &[&str]) -> Vec<Pack> {
            let all = packs::folder(&self.0.join("packs"), packs::RESOURCE_FORMAT);
            let ids: Vec<String> = names.iter().map(|n| format!("file/{n}")).collect();
            let selected = packs::enabled(&all, &ids);
            assert_eq!(selected.iter().map(|p| &p.id).collect::<Vec<_>>(), ids.iter().collect::<Vec<_>>(), "do not silently skip a fixture pack");
            assert!(selected.iter().all(|p| p.compatible));
            selected
        }
        fn server(&self, files: &[(&str, &[u8])]) -> (Pack, PathBuf, Vec<u8>) {
            // Real ZIP writer already in the official locked dependency graph.
            // A server pack is not a .hmod and does not use Mod admission.
            let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
            let options = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
            writer.start_file("pack.mcmeta", options).unwrap();
            writer.write_all(metadata(false).as_bytes()).unwrap();
            for (name, bytes) in files {
                writer.start_file(*name, options).unwrap(); writer.write_all(bytes).unwrap();
            }
            let bytes = writer.finish().unwrap().into_inner();
            let path = self.0.join("server.zip"); fs::write(&path, &bytes).unwrap();
            let pack = packs::server_pack(1, &path).expect("actual server ZIP admission");
            assert!(pack.compatible); assert!(pack.id.starts_with("server/"));
            (pack, path, bytes)
        }
        fn mod_content(&self, files: &[(&str, &[u8])]) -> (ContentMod, PathBuf, Vec<u8>) {
            let manifest = "id = \"meadow\"\nname = \"Pack chain fixture\"\nversion = \"0.0.0\"\napi = \"0.2\"\ncapabilities = [\"registries\"]\n";
            let bytes = Package::write_files(manifest, None, files).unwrap();
            let path = self.0.join("meadow.hmod"); fs::write(&path, &bytes).unwrap();
            let package = Package::read(&path).unwrap();
            let content = Content::read(&path, &package.manifest).unwrap();
            (ContentMod { id: package.manifest.id, content }, path, bytes)
        }
        fn pack_root(&self, base: &Path, selected: &[Pack], suffix: &str) -> PathBuf {
            let out = self.0.join(format!("pack-output-{suffix}"));
            packs::build_overlay(base, selected, &out).expect("actual public pack builder");
            out
        }
        fn mod_root(&self, pack_root: &Path, content: ContentMod, suffix: &str) -> PathBuf {
            let out = self.0.join(format!("mod-output-{suffix}"));
            assert!(mod_content::build(pack_root, &[content], &out).unwrap().is_empty());
            out
        }
    }
    impl Drop for Scratch {
        fn drop(&mut self) {
            let result = fs::remove_dir_all(&self.0);
            if !std::thread::panicking() { result.expect("remove only owned scratch"); }
        }
    }
    fn metadata(filter: bool) -> String {
        let (major, minor) = packs::RESOURCE_FORMAT;
        let filter = if filter { r#", "filter":{"block":[{"namespace":"meadow","path":"lang/en_us\\.json"}]}"# } else { "" };
        format!(r#"{{"pack":{{"description":"original fixture","min_format":[{major},{minor}],"max_format":[{major},{minor}]}}{filter}}}"#)
    }
    fn put(root: &Path, path: &str, bytes: &[u8]) {
        let at = root.join(path); fs::create_dir_all(at.parent().unwrap()).unwrap(); fs::write(at, bytes).unwrap();
    }
    #[derive(Debug, PartialEq, Eq)]
    enum Entry { Directory, Bytes(Vec<u8>), Link(PathBuf) }
    fn tree(root: &Path) -> BTreeMap<PathBuf, Entry> {
        fn visit(root: &Path, at: &Path, out: &mut BTreeMap<PathBuf, Entry>) {
            for entry in fs::read_dir(at).unwrap() {
                let entry = entry.unwrap(); let path = entry.path(); let ty = entry.file_type().unwrap();
                let value = if ty.is_symlink() { Entry::Link(fs::read_link(&path).unwrap()) }
                    else if ty.is_dir() { visit(root, &path, out); Entry::Directory }
                    else { assert!(ty.is_file()); Entry::Bytes(fs::read(&path).unwrap()) };
                out.insert(path.strip_prefix(root).unwrap().to_owned(), value);
            }
        }
        let mut out = BTreeMap::new(); visit(root, root, &mut out); out
    }

    #[test]
    fn selected_order_merges_keys_before_mod_whole_file_replacement() {
        let s = Scratch::new(); let base = s.base();
        s.folder("a", false, &[(EN, br#"{"gui.meadow.shared":"a","gui.meadow.a":"a only"}"#)]);
        s.folder("z", false, &[(EN, br#"{"gui.meadow.shared":"z","gui.meadow.z":"z only"}"#)]);
        let before = tree(&base); let packs_before = tree(&s.0.join("packs"));
        // Discovery sorts by name; selected order, not discovery order, wins.
        let selected = s.selected(&["z", "a"]); let root = s.pack_root(&base, &selected, "za");
        let language = Language::load(&[&root], "en_us");
        for (key, value) in [("shared", "a"), ("vanilla", "vanilla only"), ("a", "a only"), ("z", "z only")] {
            assert_eq!(language.get(&format!("gui.meadow.{key}")), Some(value));
        }
        let reversed = s.pack_root(&base, &s.selected(&["a", "z"]), "az");
        assert_eq!(Language::load(&[&reversed], "en_us").get("gui.meadow.shared"), Some("z"));
        let root_before = tree(&root); let (content, path, bytes) = s.mod_content(&[(EN, MOD)]);
        let out = s.mod_root(&root, content, "za"); let language = Language::load(&[&out], "en_us");
        assert_eq!(fs::read(out.join(EN)).unwrap(), MOD); assert_eq!(language.get("gui.meadow.shared"), Some("mod"));
        for key in ["vanilla", "a", "z"] { assert_eq!(language.get(&format!("gui.meadow.{key}")), None); }
        assert_eq!(tree(&root), root_before); assert_eq!(tree(&base), before);
        assert_eq!(tree(&s.0.join("packs")), packs_before); assert_eq!(fs::read(path).unwrap(), bytes);
    }

    #[test]
    fn supplied_server_pack_is_above_selected_packs_but_below_mod_file() {
        let s = Scratch::new(); let base = s.base();
        s.folder("user", false, &[(EN, br#"{"gui.meadow.shared":"user","gui.meadow.user":"user only"}"#)]);
        let (server, path, bytes) = s.server(&[(EN, br#"{"gui.meadow.shared":"server","gui.meadow.server":"server only"}"#)]);
        let mut selected = s.selected(&["user"]); selected.push(server);
        // This reproduces startup_root's order, not Game/server selection authority.
        let root = s.pack_root(&base, &selected, "server"); let language = Language::load(&[&root], "en_us");
        assert_eq!(language.get("gui.meadow.shared"), Some("server"));
        assert_eq!(language.get("gui.meadow.user"), Some("user only")); assert_eq!(language.get("gui.meadow.vanilla"), Some("vanilla only"));
        let (content, _, _) = s.mod_content(&[(EN, MOD)]); let out = s.mod_root(&root, content, "server");
        let language = Language::load(&[&out], "en_us"); assert_eq!(language.get("gui.meadow.shared"), Some("mod"));
        for key in ["user", "server", "vanilla"] { assert_eq!(language.get(&format!("gui.meadow.{key}")), None); }
        assert_eq!(fs::read(path).unwrap(), bytes);
    }

    #[test]
    fn filter_removes_lower_layers_but_allows_its_own_file_then_mod() {
        let s = Scratch::new(); let base = s.base();
        s.folder("lower", false, &[(EN, br#"{"gui.meadow.lower":"lower only"}"#)]);
        s.folder("filter", true, &[(EN, br#"{"gui.meadow.shared":"filter own","gui.meadow.own":"own only"}"#)]);
        let root = s.pack_root(&base, &s.selected(&["lower", "filter"]), "filtered-own");
        let language = Language::load(&[&root], "en_us");
        assert_eq!(language.get("gui.meadow.shared"), Some("filter own")); assert_eq!(language.get("gui.meadow.own"), Some("own only"));
        assert_eq!(language.get("gui.meadow.vanilla"), None); assert_eq!(language.get("gui.meadow.lower"), None);
        let (content, _, _) = s.mod_content(&[(EN, MOD)]); let out = s.mod_root(&root, content, "filtered-own");
        let language = Language::load(&[&out], "en_us"); assert_eq!(language.get("gui.meadow.shared"), Some("mod"));
        for key in ["own", "lower", "vanilla"] { assert_eq!(language.get(&format!("gui.meadow.{key}")), None); }
    }

    #[test]
    fn filter_without_replacement_stays_absent_until_explicit_mod_file() {
        let s = Scratch::new(); let base = s.base();
        s.folder("filter", true, &[]); let root = s.pack_root(&base, &s.selected(&["filter"]), "absent");
        assert!(!root.join(EN).exists()); assert_eq!(Language::load(&[&root], "en_us").get("gui.meadow.vanilla"), None);
        let (content, _, _) = s.mod_content(&[(EN, MOD)]); let out = s.mod_root(&root, content, "absent");
        assert_eq!(fs::read(out.join(EN)).unwrap(), MOD);
        let language = Language::load(&[&out], "en_us"); assert_eq!(language.get("gui.meadow.shared"), Some("mod"));
        assert_eq!(language.get("gui.meadow.vanilla"), None, "do not resurrect filtered source keys");
    }

    #[test]
    fn invalid_pack_layer_is_skipped_but_invalid_mod_file_replaces() {
        let s = Scratch::new(); let base = s.base();
        put(&base, JA, br#"{"gui.meadow.shared":"selected base"}"#);
        s.folder("broken", false, &[(EN, b"not json"), (JA, b"not json")]);
        let root = s.pack_root(&base, &s.selected(&["broken"]), "invalid");
        assert_eq!(Language::load(&[&root], "en_us").get("gui.meadow.shared"), Some("vanilla"));
        assert_eq!(Language::load(&[&root], "ja_jp").get("gui.meadow.shared"), Some("selected base"));
        let (content, _, _) = s.mod_content(&[(JA, b"not json")]); let out = s.mod_root(&root, content, "invalid");
        assert_eq!(fs::read(out.join(JA)).unwrap(), b"not json");
        assert_eq!(Language::load(&[&out], "ja_jp").get("gui.meadow.shared"), Some("vanilla"));
    }

    #[test]
    fn unchanged_pack_cache_and_selected_locale_survive_mod_english_overlay() {
        let s = Scratch::new(); let base = s.base();
        s.folder("selected", false, &[(JA, "{\"gui.meadow.shared\":\"パック\",\"gui.meadow.ja\":\"日本語\"}".as_bytes())]);
        let selected = s.selected(&["selected"]); let root = s.pack_root(&base, &selected, "reuse"); let before = tree(&root);
        // Keep actual cache validation/locking. Do not call the private uncached helper.
        packs::build_overlay(&base, &selected, &root).unwrap(); assert_eq!(tree(&root), before);
        let (content, _, _) = s.mod_content(&[(EN, MOD)]); let out = s.mod_root(&root, content, "reuse");
        let language = Language::load(&[&out], "ja_jp"); assert_eq!(language.get("gui.meadow.shared"), Some("パック"));
        assert_eq!(language.get("gui.meadow.mod"), Some("mod only")); assert_eq!(language.get("gui.meadow.ja"), Some("日本語"));
        assert_eq!(Language::load(&[&out], "ja").get("gui.meadow.shared"), Some("mod"));
        assert_eq!(tree(&root), before);
    }
}
