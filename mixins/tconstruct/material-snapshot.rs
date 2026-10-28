use std::collections::BTreeMap;

// Keep every material definition, stat, trait and material-tag file in pack
// order. Definitions select the highest layer; stats and traits merge
// fields across layers. Parsing and gameplay remain unfinished.
pub(super) fn merge_pack(
    materials: &mut BTreeMap<String, Vec<Vec<u8>>>,
    tags: &mut BTreeMap<String, Vec<Vec<u8>>>,
    files: impl IntoIterator<Item = String>,
    read: impl Fn(&str) -> Option<Vec<u8>>,
) {
    for file in files {
        let Some(rest) = file.strip_prefix("data/") else { continue; };
        let Some((namespace, path)) = rest.split_once('/') else { continue; };
        let Some(path) = path.strip_suffix(".json") else { continue; };
        if namespace.is_empty() || !namespace.bytes().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || b"_.-".contains(&c))
            || !path.bytes().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || b"_./-".contains(&c))
            || path.split('/').any(|part| part.is_empty() || part == "." || part == "..") { continue; }
        let is_tag = path.starts_with("tinkering/tags/materials/");
        let is_material = ["definition", "stats", "traits"].iter().any(|kind| path.starts_with(&format!("tinkering/materials/{kind}/")));
        if !is_tag && !is_material { continue; }
        if let Some(bytes) = read(&file) {
            let id = format!("{namespace}:{path}");
            if is_tag { tags.entry(id).or_default().push(bytes); }
            else { materials.entry(id).or_default().push(bytes); }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn material_and_tag_files_preserve_all_pack_layers() {
        let material = "data/tconstruct/tinkering/materials/definition/wood.json";
        let tag = "data/tconstruct/tinkering/tags/materials/wood.json";
        let mut values = BTreeMap::new(); let mut tags = BTreeMap::new();
        merge_pack(&mut values, &mut tags, [material.into(), tag.into()], |_| Some(b"base".to_vec()));
        merge_pack(&mut values, &mut tags, [material.into(), tag.into()], |_| Some(b"override".to_vec()));
        assert_eq!(values["tconstruct:tinkering/materials/definition/wood"], [b"base".to_vec(), b"override".to_vec()]);
        assert_eq!(tags["tconstruct:tinkering/tags/materials/wood"], [b"base".to_vec(), b"override".to_vec()]);
    }
}
