// Excluded candidate. Not registered in a Pluto bundle or applied to any host.
// Preparation only: no Language/CURRENT, pack selection, callback, or cache owner.
use serde::de::{self, MapAccess, Visitor};
use serde::Deserialize;
use std::{borrow::Cow, collections::BTreeMap, fmt, fs::File, io::{self, Read, Write}, path::Path};

pub(super) const POLICY_ID: &str = "plutolib language lower-only strict strings 1";
const MAX_INPUT_BYTES: usize = 1024 * 1024;
const MAX_ENTRIES: usize = 16_384;
const MAX_DECODED_BYTES: usize = 2 * MAX_INPUT_BYTES;
const MAX_OUTPUT_BYTES: usize = 2 * MAX_INPUT_BYTES;

struct Strings(BTreeMap<String, String>);
impl<'de> Deserialize<'de> for Strings {
    fn deserialize<D: de::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct Flat;
        impl<'de> Visitor<'de> for Flat {
            type Value = Strings;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a bounded object with unique string keys and string values")
            }
            fn visit_map<A: MapAccess<'de>>(self, mut a: A) -> Result<Strings, A::Error> {
                let mut map = BTreeMap::new();
                let mut decoded = 0usize;
                while let Some(key) = a.next_key::<String>()? {
                    if map.len() == MAX_ENTRIES || map.contains_key(&key) {
                        return Err(de::Error::custom("entry limit or duplicate key"));
                    }
                    let value = a.next_value::<String>()?;
                    decoded = decoded.checked_add(key.len()).and_then(|n| n.checked_add(value.len()))
                        .filter(|n| *n <= MAX_DECODED_BYTES)
                        .ok_or_else(|| de::Error::custom("decoded text limit"))?;
                    map.insert(key, value);
                }
                Ok(Strings(map))
            }
        }
        d.deserialize_map(Flat)
    }
}

fn parse(bytes: &[u8]) -> Option<BTreeMap<String, String>> {
    if bytes.len() > MAX_INPUT_BYTES { return None; }
    serde_json::from_slice::<Strings>(bytes).ok().map(|v| v.0)
}

// Exact four-component locale asset path only. No path/locale normalization.
fn language_path(rel: &str) -> bool {
    let mut p = rel.split('/');
    let (Some("assets"), Some(namespace), Some("lang"), Some(file), None) =
        (p.next(), p.next(), p.next(), p.next(), p.next()) else { return false; };
    let Some(locale) = file.strip_suffix(".json") else { return false; };
    !namespace.is_empty() && !matches!(namespace, "." | "..") &&
        namespace.bytes().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || b"_.-".contains(&c)) &&
        !locale.is_empty() && locale.bytes().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || b"_-".contains(&c))
}

struct BoundedOutput(Vec<u8>);
impl Write for BoundedOutput {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if self.0.len().checked_add(bytes.len()).is_none_or(|n| n > MAX_OUTPUT_BYTES) {
            return Err(io::Error::other("encoded language limit"));
        }
        self.0.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> { Ok(()) }
}

fn combine(lower: BTreeMap<String, String>, higher: BTreeMap<String, String>) -> Option<Vec<u8>> {
    // With no additional key, keep the original higher bytes, including layout.
    if lower.keys().all(|key| higher.contains_key(key)) { return None; }
    let mut result = lower;
    result.extend(higher); // Existing native Mod-wins value precedence.
    if result.len() > MAX_ENTRIES { return None; }
    let decoded = result.iter().try_fold(0usize, |n, (k, v)| n.checked_add(k.len())?.checked_add(v.len()));
    if decoded.is_none_or(|n| n > MAX_DECODED_BYTES) { return None; }
    let mut out = BoundedOutput(Vec::new());
    serde_json::to_writer(&mut out, &result).ok()?;
    Some(out.0)
}

fn lower_bytes(path: &Path) -> Option<Vec<u8>> {
    // Native prepared roots may contain symlinks. Do not detach or rewrite them.
    // A quiescent admitted root is assumed, not hostile-filesystem race safety.
    let metadata = path.metadata().ok()?;
    if !metadata.is_file() || metadata.len() > MAX_INPUT_BYTES as u64 { return None; }
    let mut bytes = Vec::new();
    File::open(path).ok()?.take((MAX_INPUT_BYTES + 1) as u64).read_to_end(&mut bytes).ok()?;
    (bytes.len() <= MAX_INPUT_BYTES).then_some(bytes)
}

// Called only after the native admitted-package reader succeeded, immediately
// before its existing Overlay::write. Its directory/link/write logic stays native.
pub(super) fn prepare<'a>(base: &Path, rel: &str, higher: &'a [u8]) -> Cow<'a, [u8]> {
    if !language_path(rel) { return Cow::Borrowed(higher); }
    let merged = (|| {
        let higher = parse(higher)?;
        let lower = lower_bytes(&base.join(rel))?;
        combine(parse(&lower)?, higher)
    })();
    merged.map_or(Cow::Borrowed(higher), Cow::Owned)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, path::PathBuf, sync::atomic::{AtomicU64, Ordering}};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    struct Scratch(PathBuf);
    impl Scratch {
        fn new() -> Self {
            for _ in 0..128 {
                let path = std::env::temp_dir().join(format!("pluto-language-repair-{}-{}", std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed)));
                match fs::create_dir(&path) {
                    Ok(()) => return Self(path),
                    Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
                    Err(e) => panic!("owned scratch: {e}"),
                }
            }
            panic!("owned scratch names exhausted")
        }
        fn put(&self, bytes: &[u8]) -> PathBuf {
            let path = self.0.join("assets/meadow/lang/en_us.json");
            fs::create_dir_all(path.parent().unwrap()).unwrap(); fs::write(&path, bytes).unwrap(); path
        }
    }
    impl Drop for Scratch {
        fn drop(&mut self) {
            let result = fs::remove_dir_all(&self.0);
            if !std::thread::panicking() { result.expect("remove owned scratch only"); }
        }
    }

    fn merged(lower: &[u8], higher: &[u8]) -> Option<Vec<u8>> {
        combine(parse(lower)?, parse(higher)?)
    }

    #[test]
    fn adds_only_lower_keys_and_keeps_mod_conflicts() {
        let bytes = merged(br#"{"a":"lower","b":"lower only"}"#, br#"{"a":"mod","c":"mod only"}"#).unwrap();
        assert_eq!(bytes, br#"{"a":"mod","b":"lower only","c":"mod only"}"#);
    }
    #[test]
    fn unchanged_key_set_requests_original_higher_bytes() {
        assert!(merged(br#"{"a":"lower"}"#, br#"{ "a" : "mod", "b":"own" }"#).is_none());
        assert!(merged(b"{}", b"{}").is_none());
    }
    #[test]
    fn empty_higher_retains_lower_keys_by_explicit_policy() {
        assert_eq!(merged(br#"{"a":"lower"}"#, b"{}").unwrap(), br#"{"a":"lower"}"#);
    }
    #[test]
    fn duplicates_in_either_layer_are_not_eligible() {
        for duplicate in [br#"{"x":"a","x":"b"}"#.as_slice(), br#"{"x":"a","\u0078":"b"}"#] {
            assert!(merged(duplicate, b"{}").is_none());
            assert!(merged(br#"{"k":"v"}"#, duplicate).is_none());
        }
    }
    #[test]
    fn unsupported_and_partial_documents_remain_ineligible() {
        for invalid in [b"".as_slice(), b"[]", b"null", br#"{"x":1}"#, br#"{"x":true}"#,
                        br#"{"x":null}"#, br#"{"x":[]}"#, br#"{"x":{}}"#,
                        br#"{"a":"ok","b":null}"#, br#"{"a":"ok"} {}"#, b"#comment\n{\"a\":\"ok\"}"] {
            assert!(parse(invalid).is_none());
        }
    }
    #[test]
    fn unicode_and_placeholders_are_values_not_format_programs() {
        let lower = "{\"emoji\":\"🌿\",\"line\":\"a\\nb\"}";
        let higher = r#"{"format":"%d %s %1$s %% {name}"}"#;
        let result = parse(&merged(lower.as_bytes(), higher.as_bytes()).unwrap()).unwrap();
        assert_eq!(result["emoji"], "🌿"); assert_eq!(result["line"], "a\nb");
        assert_eq!(result["format"], "%d %s %1$s %% {name}");
    }
    #[test]
    fn exact_input_byte_limit_is_enforced() {
        let mut exact = vec![b' '; MAX_INPUT_BYTES - 2]; exact.extend_from_slice(b"{}");
        assert!(parse(&exact).is_some()); exact.push(b' '); assert!(parse(&exact).is_none());
    }
    #[test]
    fn parser_and_union_entry_limits_are_enforced() {
        let full: BTreeMap<_, _> = (0..MAX_ENTRIES).map(|i| (i.to_string(), String::new())).collect();
        assert_eq!(parse(&serde_json::to_vec(&full).unwrap()).unwrap().len(), MAX_ENTRIES);
        let extra = BTreeMap::from([("extra".to_owned(), String::new())]);
        assert!(combine(full.clone(), extra.clone()).is_none());
        let mut too_many = full; too_many.extend(extra);
        assert!(parse(&serde_json::to_vec(&too_many).unwrap()).is_none());
    }
    #[test]
    fn encoded_writer_never_grows_past_limit() {
        let mut out = BoundedOutput(vec![0; MAX_OUTPUT_BYTES - 1]);
        assert!(out.write_all(b"12").is_err()); assert_eq!(out.0.len(), MAX_OUTPUT_BYTES - 1);
        out.write_all(b"1").unwrap(); assert_eq!(out.0.len(), MAX_OUTPUT_BYTES);
    }
    #[test]
    fn path_scope_rejects_other_resources_and_traversal() {
        assert!(language_path("assets/meadow/lang/en_us.json"));
        assert!(language_path("assets/my.mod-2/lang/ja_jp.json"));
        for path in ["assets/meadow/lang/en_us.toml", "assets/meadow/lang/nested/en_us.json",
                     "assets/../lang/en_us.json", "/assets/meadow/lang/en_us.json", "assets/meadow/lang/../en_us.json",
                     "data/meadow/lang/en_us.json", "assets/meadow/lang/.json", "assets/meadow/lang/EN_US.json",
                     "assets/meadow/lang/en_us.json/", "assets\\meadow\\lang\\en_us.json"] {
            assert!(!language_path(path), "{path}");
        }
    }
    #[test]
    fn missing_lower_and_ineligible_higher_keep_exact_borrow() {
        let scratch = Scratch::new();
        let higher = b" { \"x\" : \"original\" } ";
        let path = &scratch.0;
        for rel in ["assets/meadow/lang/en_us.json", "assets/meadow/textures/x.png"] {
            let result = prepare(path, rel, higher);
            assert!(matches!(result, Cow::Borrowed(_))); assert_eq!(result.as_ptr(), higher.as_ptr());
        }
        let unsupported = br#"{"x":false}"#;
        assert!(matches!(prepare(path, "assets/meadow/lang/en_us.json", unsupported), Cow::Borrowed(_)));
    }
    #[test]
    fn native_prepared_lower_is_read_only_and_mod_wins() {
        let scratch = Scratch::new();
        let lower = br#"{"only":"base","shared":"pack"}"#;
        let path = scratch.put(lower);
        let higher = br#"{"shared":"mod"}"#;
        let result = prepare(&scratch.0, "assets/meadow/lang/en_us.json", higher);
        assert!(matches!(result, Cow::Owned(_)));
        assert_eq!(result.as_ref(), br#"{"only":"base","shared":"mod"}"#);
        assert_eq!(fs::read(path).unwrap(), lower);
        assert_eq!(higher, br#"{"shared":"mod"}"#);
    }
    #[test]
    fn invalid_or_large_lower_preserves_original_higher() {
        let scratch = Scratch::new(); let higher = b" { } ";
        for lower in [br#"{"a":false}"#.to_vec(), b"invalid".to_vec(), vec![b' '; MAX_INPUT_BYTES + 1]] {
            scratch.put(&lower);
            let result = prepare(&scratch.0, "assets/meadow/lang/en_us.json", higher);
            assert!(matches!(result, Cow::Borrowed(_))); assert_eq!(result.as_ptr(), higher.as_ptr());
        }
    }
    #[test]
    fn lower_directory_is_not_opened_as_a_file() {
        let scratch = Scratch::new();
        fs::create_dir_all(scratch.0.join("assets/meadow/lang/en_us.json")).unwrap();
        assert!(matches!(prepare(&scratch.0, "assets/meadow/lang/en_us.json", b"{}"), Cow::Borrowed(_)));
    }
}
