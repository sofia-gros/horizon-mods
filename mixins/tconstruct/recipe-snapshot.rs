use std::collections::BTreeMap;

/// Merge one pack's original-format recipe files by namespaced ID. Higher
/// priority packs call this later. Invalid names are ignored; bytes stay raw
/// until the source recipe loader validates them. This is not yet used for
/// selecting or taking Part Builder recipes.
pub(super) fn merge_pack(
    recipes: &mut BTreeMap<String, Vec<u8>>,
    files: impl IntoIterator<Item = String>,
    read: impl Fn(&str) -> Option<Vec<u8>>,
) {
    for file in files {
        let Some(rest) = file.strip_prefix("data/") else { continue; };
        let Some((namespace, path)) = rest.split_once('/') else { continue; };
        let Some(path) = path.strip_prefix("recipes/").and_then(|p| p.strip_suffix(".json")) else { continue; };
        if namespace.is_empty() || path.is_empty() || path.starts_with('_')
            || !namespace.bytes().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || b"_.-".contains(&c))
            || !path.bytes().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || b"_./-".contains(&c))
            || path.split('/').any(|part| part == "." || part == ".." || part.is_empty()) { continue; }
        if let Some(bytes) = read(&file) {
            recipes.insert(format!("{namespace}:{path}"), bytes);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn later_pack_overrides_by_id_and_world_snapshots_are_independent() {
        let path = "data/tconstruct/recipes/tools/materials/flint/flint.json";
        let mut first = BTreeMap::new();
        let mut second = BTreeMap::new();
        merge_pack(&mut first, [path.into()], |_| Some(b"base".to_vec()));
        merge_pack(&mut second, [path.into()], |_| Some(b"base".to_vec()));
        merge_pack(&mut first, [path.into(), "data/tconstruct/recipes/../bad.json".into()], |name| Some(if name == path { b"override".to_vec() } else { b"bad".to_vec() }));
        assert_eq!(first.get("tconstruct:tools/materials/flint/flint"), Some(&b"override".to_vec()));
        assert_eq!(second.get("tconstruct:tools/materials/flint/flint"), Some(&b"base".to_vec()));
        assert_eq!(first.len(), 1);
    }
}
