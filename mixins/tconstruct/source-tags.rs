// Forge 1.20.1 tag file merge, before registry-dependent resolution.
// A tag's files are supplied in increasing pack priority. This module is not
// yet connected to gameplay or world pack enumeration.
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct TagEntry {
    pub id: String,
    pub required: bool,
    pub remove: bool,
}

/// Invalid tag files are ignored as a whole, as in Forge's TagLoader.
pub(super) fn merge_tag_files(files: &[Vec<u8>]) -> Vec<TagEntry> {
    let mut merged = Vec::new();
    for bytes in files {
        let Some((replace, entries)) = parse_file(bytes) else { continue; };
        if replace { merged.clear(); }
        merged.extend(entries);
    }
    merged
}

fn parse_file(bytes: &[u8]) -> Option<(bool, Vec<TagEntry>)> {
    let value: serde_json::Value = serde_json::from_slice(bytes).ok()?;
    let obj = value.as_object()?;
    let replace = match obj.get("replace") {
        None => false,
        Some(value) => value.as_bool()?,
    };
    let mut entries = Vec::new();
    for (key, remove) in [("values", false), ("remove", true)] {
        let values = match obj.get(key) {
            None if remove => continue,
            value => value?.as_array()?,
        };
        for value in values {
            let (id, required) = if let Some(id) = value.as_str() {
                (id, true)
            } else {
                let item = value.as_object()?;
                let id = item.get("id")?.as_str()?;
                let required = match item.get("required") {
                    Some(required) => required.as_bool()?,
                    None => true,
                };
                (id, required)
            };
            if !valid_id(id.strip_prefix('#').unwrap_or(id)) { return None; }
            entries.push(TagEntry { id: id.to_owned(), required, remove });
        }
    }
    Some((replace, entries))
}

fn valid_id(id: &str) -> bool {
    let (namespace, path) = id.split_once(':').unwrap_or(("minecraft", id));
    !namespace.is_empty() && !path.is_empty()
        && namespace.bytes().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || b"_.-".contains(&c))
        && path.bytes().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || b"_./-".contains(&c))
}

/// Reconstruct the ordered merge inputs by namespaced tag ID. Values are
/// raw files so `replace` and Forge `remove` retain their original ordering.
pub(super) fn merge_pack_tags(
    tags: &mut BTreeMap<String, Vec<Vec<u8>>>,
    files: impl IntoIterator<Item = String>,
    read: impl Fn(&str) -> Option<Vec<u8>>,
) {
    for file in files {
        let Some(rest) = file.strip_prefix("data/") else { continue; };
        let Some((namespace, path)) = rest.split_once('/') else { continue; };
        let Some(path) = path.strip_prefix("tags/items/").and_then(|p| p.strip_suffix(".json")) else { continue; };
        if !valid_id(&format!("{namespace}:{path}")) || path.split('/').any(|p| p.is_empty() || p == "." || p == "..") { continue; }
        if let Some(bytes) = read(&file) { tags.entry(format!("{namespace}:{path}")).or_default().push(bytes); }
    }
}

/// Resolve an acyclic tag against the current registry. Missing required
/// entries fail the tag; missing removals are optional in Forge. Cycles are
/// explicitly unsupported for now and must not be treated as an empty tag.
#[allow(dead_code)]
pub(super) fn resolve_tag(
    id: &str,
    tags: &BTreeMap<String, Vec<Vec<u8>>>,
    registry: &std::collections::HashSet<String>,
) -> Result<Vec<String>, String> {
    fn canonical(id: &str) -> String {
        if id.contains(':') { id.to_owned() } else { format!("minecraft:{id}") }
    }
    fn resolve(
        id: &str,
        tags: &BTreeMap<String, Vec<Vec<u8>>>,
        registry: &std::collections::HashSet<String>,
        visiting: &mut Vec<String>,
    ) -> Result<Vec<String>, String> {
        if visiting.iter().any(|v| v == id) { return Err(format!("cyclic tag {id}")); }
        let files = tags.get(id).ok_or_else(|| format!("missing tag {id}"))?;
        if !files.iter().any(|file| parse_file(file).is_some()) { return Err(format!("invalid tag {id}")); }
        visiting.push(id.to_owned());
        let mut result: Vec<String> = Vec::new();
        for entry in merge_tag_files(files) {
            let found = if let Some(tag) = entry.id.strip_prefix('#') {
                resolve(&canonical(tag), tags, registry, visiting)
            } else {
                let item = canonical(&entry.id);
                if registry.contains(&item) { Ok(vec![item]) } else { Err(format!("missing item {item}")) }
            };
            let values = match found {
                Ok(values) => values,
                // A cyclic dependency is not safely resolved by treating it as optional.
                Err(error) if error.starts_with("cyclic tag ") => { visiting.pop(); return Err(error); },
                Err(_) if entry.remove || !entry.required => continue,
                Err(error) => { visiting.pop(); return Err(error); },
            };
            for value in values {
                if entry.remove { result.retain(|v| v != &value); }
                else if !result.contains(&value) { result.push(value); }
            }
        }
        visiting.pop();
        Ok(result)
    }
    resolve(&canonical(id), tags, registry, &mut Vec::new())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pack_priority_replace_and_forge_removal() {
        let files = vec![br##"{"values":["minecraft:stone",{"id":"#test:optional","required":false}]}"##.to_vec(),
                         br##"{"replace":true,"values":["test:wood"],"remove":["#test:old"]}"##.to_vec()];
        assert_eq!(merge_tag_files(&files), vec![TagEntry { id:"test:wood".into(), required:true, remove:false }, TagEntry { id:"#test:old".into(), required:true, remove:true }]);
    }
    #[test]
    fn invalid_files_do_not_erase_lower_priority() {
        let files = vec![br#"{"values":["test:keep"]}"#.to_vec(), br#"{"replace":true,"values":[42]}"#.to_vec()];
        assert_eq!(merge_tag_files(&files), vec![TagEntry { id:"test:keep".into(), required:true, remove:false }]);
    }
    #[test]
    fn required_optional_nested_and_removals() {
        let mut tags = BTreeMap::new();
        tags.insert("test:base".into(), vec![br#"{"values":["test:a","test:b"]}"#.to_vec()]);
        tags.insert("test:result".into(), vec![br##"{"values":["#test:base",{"id":"test:missing","required":false},"test:c"],"remove":["test:b","test:missing"]}"##.to_vec()]);
        let registry = ["test:a", "test:b", "test:c"].into_iter().map(str::to_owned).collect();
        assert_eq!(resolve_tag("test:result", &tags, &registry).unwrap(), ["test:a", "test:c"]);
        tags.insert("test:bad".into(), vec![br#"{"values":["test:missing"]}"#.to_vec()]);
        assert!(resolve_tag("test:bad", &tags, &registry).is_err());
    }
    #[test]
    fn cyclic_tags_are_not_silently_accepted() {
        let mut tags = BTreeMap::new();
        tags.insert("test:cycle".into(), vec![br##"{"values":[{"id":"#test:cycle","required":false}]}"##.to_vec()]);
        assert!(resolve_tag("test:cycle", &tags, &Default::default()).unwrap_err().starts_with("cyclic tag"));
    }
    #[test]
    fn raw_tag_files_preserve_pack_order_and_ignore_other_kinds() {
        let mut tags = BTreeMap::new();
        let path = "data/test/tags/items/a.json";
        merge_pack_tags(&mut tags, [path.into(), "data/test/tags/blocks/a.json".into()], |_| Some(b"low".to_vec()));
        merge_pack_tags(&mut tags, [path.into()], |_| Some(b"high".to_vec()));
        assert_eq!(tags["test:a"], [b"low".to_vec(), b"high".to_vec()]);
        assert_eq!(tags.len(), 1);
    }
}
