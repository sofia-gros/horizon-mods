//! Ordered raw data-pack JSON resources, without registry or format policy.
//!
//! Extracted from TConstruct's modifier snapshot at horizon-mods commit
//! 1a75477e7e08e024e77112a6aa8ad764e29023eb. Consumers choose resource paths,
//! supply packs in increasing priority, and interpret the bytes themselves.
use std::collections::BTreeMap;

/// Resource ID to layers in caller-supplied pack order, lowest priority first.
pub type Layers = BTreeMap<String, Vec<Vec<u8>>>;

/// A validated path beneath a pack's `data` directory, without `.json`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DataPath<'a> {
    pub namespace: &'a str,
    pub path: &'a str,
}

impl DataPath<'_> {
    pub fn id(self) -> String {
        format!("{}:{}", self.namespace, self.path)
    }
}

/// Parse only canonical relative JSON resource paths. This is a resource-name
/// check, not a filesystem sandbox: the consumer owns all actual reads.
/// Unknown directories are accepted so each consumer can select its own kind.
pub fn data_json_path(file: &str) -> Option<DataPath<'_>> {
    let rest = file.strip_prefix("data/")?;
    let (namespace, path) = rest.split_once('/')?;
    let path = path.strip_suffix(".json")?;
    if namespace.is_empty()
        || !namespace
            .bytes()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || b"_.-".contains(&c))
        || !path
            .bytes()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || b"_./-".contains(&c))
        || path
            .split('/')
            .any(|part| part.is_empty() || part == "." || part == "..")
    {
        return None;
    }
    Some(DataPath { namespace, path })
}

/// A listed, selected resource could not be read. Do not use a partially
/// collected snapshot as an authoritative registry after this error.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnreadableResource {
    pub file: String,
}

impl std::fmt::Display for UnreadableResource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "unreadable resource {}", self.file)
    }
}
impl std::error::Error for UnreadableResource {}

/// Whether a listed, selected resource that cannot be read invalidates the
/// traversal. Consumers must choose according to their snapshot contract.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnreadablePolicy {
    /// Stop at the first unreadable selected file. Prior visits remain visible.
    Error,
    /// Omit that file and continue. This does not establish snapshot completeness.
    Skip,
}

/// Visit selected JSON resources in their supplied order. Invalid paths and
/// resources rejected by `classify` are never read. The classification value
/// lets consumers route a resource without repeating their path policy.
///
/// Bytes are passed through unchanged, including malformed JSON. Earlier
/// visits remain visible if a later read fails; callers must discard or mark
/// their whole snapshot unavailable on error. This function neither sorts nor
/// deduplicates entries and cannot determine whether an ID is registered.
pub fn visit_json_pack<K>(
    files: impl IntoIterator<Item = String>,
    read: impl FnMut(&str) -> Option<Vec<u8>>,
    classify: impl FnMut(DataPath<'_>) -> Option<K>,
    visit: impl FnMut(K, String, Vec<u8>),
) -> Result<(), UnreadableResource> {
    visit_json_pack_with_policy(files, read, classify, visit, UnreadablePolicy::Error)
}

/// Visit selected raw resources with an explicit unreadable-file policy.
/// `Skip` continues after missing reads and always returns `Ok(())`; it must
/// not be used when omission would falsely establish registry completeness.
/// This does not parse bytes or change the supplied file/pack order. Invalid
/// names and unselected resources are still rejected before any read.
pub fn visit_json_pack_with_policy<K>(
    files: impl IntoIterator<Item = String>,
    mut read: impl FnMut(&str) -> Option<Vec<u8>>,
    mut classify: impl FnMut(DataPath<'_>) -> Option<K>,
    mut visit: impl FnMut(K, String, Vec<u8>),
    unreadable: UnreadablePolicy,
) -> Result<(), UnreadableResource> {
    for file in files {
        let Some(path) = data_json_path(&file) else {
            continue;
        };
        let Some(kind) = classify(path) else { continue };
        let Some(bytes) = read(&file) else {
            match unreadable {
                UnreadablePolicy::Error => return Err(UnreadableResource { file }),
                UnreadablePolicy::Skip => continue,
            }
        };
        visit(kind, path.id(), bytes);
    }
    Ok(())
}

/// Select the last supplied layer. Does not parse JSON, test conditions,
/// resolve redirects, merge tags, or fall back when the selected bytes are bad.
pub fn highest<'a>(layers: &'a Layers, id: &str) -> Option<&'a [u8]> {
    layers.get(id)?.last().map(Vec::as_slice)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_paths_are_generic_and_invalid_paths_are_rejected() {
        let path = data_json_path("data/other.mod/my_kind/nested/value.json").unwrap();
        assert_eq!(path.id(), "other.mod:my_kind/nested/value");
        for file in [
            "assets/x/a.json",
            "/data/x/a.json",
            "data//a.json",
            "data/Upper/a.json",
            "data/x/A.json",
            "data/x/.json",
            "data/x/a//b.json",
            "data/x/../a.json",
            "data/x/./a.json",
            "data/x/a\\b.json",
            "data/x/a.json/",
            "data/x/a.txt",
        ] {
            assert_eq!(data_json_path(file), None, "{file}");
        }
    }

    #[test]
    fn selection_runs_before_reads_and_preserves_raw_bytes_and_order() {
        let files = [
            "data/x/one/a.json",
            "data/x/other/a.json",
            "data/x/../a.json",
        ];
        let mut reads = Vec::new();
        let mut layers = Layers::new();
        for raw in [b"lower".as_slice(), b"not JSON".as_slice()] {
            visit_json_pack(
                files.map(str::to_owned),
                |file| {
                    reads.push(file.to_owned());
                    Some(raw.to_vec())
                },
                |path| path.path.starts_with("one/").then_some(()),
                |(), id, bytes| layers.entry(id).or_default().push(bytes),
            )
            .unwrap();
        }
        assert_eq!(reads, [files[0], files[0]]);
        assert_eq!(layers["x:one/a"], [b"lower".to_vec(), b"not JSON".to_vec()]);
        assert_eq!(highest(&layers, "x:one/a"), Some(b"not JSON".as_slice()));
        assert_eq!(highest(&layers, "x:missing"), None);
        layers.insert("x:empty".into(), vec![]);
        assert_eq!(highest(&layers, "x:empty"), None);
    }

    #[test]
    fn unreadable_selected_resource_stops_without_silent_omission() {
        let files = ["data/x/one.json", "data/x/two.json", "data/x/three.json"];
        let mut visited = Vec::new();
        let error = visit_json_pack(
            files.map(str::to_owned),
            |file| (file != files[1]).then(|| vec![0, 255]),
            |_| Some(()),
            |(), id, bytes| visited.push((id, bytes)),
        )
        .unwrap_err();
        assert_eq!(error.file, files[1]);
        assert_eq!(visited, [("x:one".into(), vec![0, 255])]);
    }

    #[test]
    fn caller_classification_routes_multiple_resource_kinds() {
        let files = ["data/x/definitions/a.json", "data/x/tags/a.json"];
        let mut buckets = [Layers::new(), Layers::new()];
        visit_json_pack(
            files.map(str::to_owned),
            |_| Some(vec![42]),
            |path| match path.path.split('/').next()? {
                "definitions" => Some(0),
                "tags" => Some(1),
                _ => None,
            },
            |bucket, id, bytes| buckets[bucket].entry(id).or_default().push(bytes),
        )
        .unwrap();
        assert_eq!(buckets[0].keys().collect::<Vec<_>>(), ["x:definitions/a"]);
        assert_eq!(buckets[1].keys().collect::<Vec<_>>(), ["x:tags/a"]);
    }

    #[test]
    fn explicit_skip_continues_after_unreadable_selected_resources() {
        let files = [
            "data/x/values/first.json",
            "data/x/values/missing.json",
            "data/x/other/ignored.json",
            "data/x/values/../invalid.json",
            "data/x/values/last.json",
        ];
        let mut reads = Vec::new();
        let mut visited = Vec::new();
        visit_json_pack_with_policy(
            files.map(str::to_owned),
            |file| {
                reads.push(file.to_owned());
                (file != files[1]).then(|| vec![0, 255])
            },
            |path| path.path.starts_with("values/").then_some(()),
            |(), id, bytes| visited.push((id, bytes)),
            UnreadablePolicy::Skip,
        ).unwrap();
        assert_eq!(reads, [files[0], files[1], files[4]]);
        assert_eq!(visited, [
            ("x:values/first".into(), vec![0, 255]),
            ("x:values/last".into(), vec![0, 255]),
        ]);
    }
}
