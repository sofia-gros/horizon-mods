// Raw modifier resources for a future book renderer.
use std::collections::{BTreeMap, HashSet};

pub(super) type Layers = BTreeMap<String, Vec<Vec<u8>>>;
pub(super) type Raw = (Layers, Layers); // definitions, then modifier tags

/// Merge in increasing pack priority. A listed but unreadable file makes the
/// snapshot unavailable rather than returning a silently incomplete registry.
pub(super) fn merge_pack(
    snapshot: &mut Raw,
    files: impl IntoIterator<Item = String>,
    read: impl Fn(&str) -> Option<Vec<u8>>,
) -> Result<(), String> {
    for file in files {
        let Some(rest) = file.strip_prefix("data/") else { continue; };
        let Some((namespace, path)) = rest.split_once('/') else { continue; };
        let Some(path) = path.strip_suffix(".json") else { continue; };
        if namespace.is_empty() || !namespace.bytes().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || b"_.-".contains(&c))
            || !path.bytes().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || b"_./-".contains(&c))
            || path.split('/').any(|part| part.is_empty() || part == "." || part == "..") { continue; }
        let is_definition = path.starts_with("tinkering/modifiers/");
        let is_tag = path.starts_with("tinkering/tags/modifiers/");
        if !is_definition && !is_tag { continue; }
        let bytes = read(&file).ok_or_else(|| format!("unreadable modifier resource {file}"))?;
        let id = format!("{namespace}:{path}");
        if is_tag { snapshot.1.entry(id).or_default().push(bytes); }
        else { snapshot.0.entry(id).or_default().push(bytes); }
    }
    Ok(())
}

/// Source dynamic definitions select the highest-priority file; retaining the
/// layers also lets a future consumer diagnose or re-evaluate conditions.
#[allow(dead_code)]
pub(super) fn highest_definition<'a>(snapshot: &'a Raw, id: &str) -> Option<&'a [u8]> {
    snapshot.0.get(id)?.last().map(Vec::as_slice)
}

/// Resolve one tag only with a complete authoritative modifier registry. A
/// file name alone is not proof of registration: definitions may be invalid,
/// conditional, redirects, or shadow a static modifier. Missing registry is
/// explicitly unknown; no caller currently supplies a complete registry.
#[allow(dead_code)]
pub(super) fn resolve_tag(snapshot: &Raw, tag: &str, complete_registry: Option<&HashSet<String>>) -> Result<Vec<String>, String> {
    let registry = complete_registry.ok_or("modifier registry unavailable")?;
    let mut tags = BTreeMap::new();
    for (id, layers) in &snapshot.1 {
        let Some((namespace, path)) = id.split_once(':') else { return Err(format!("invalid modifier tag {id}")); };
        let Some(path) = path.strip_prefix("tinkering/tags/modifiers/") else { continue; };
        tags.insert(format!("{namespace}:{path}"), layers.clone());
    }
    super::tconstruct_source_tags::resolve_tag(tag, &tags, registry)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn capture_is_scoped_ordered_and_fails_on_unreadable_files() {
        let mut raw = Raw::default();
        let paths = ["data/tconstruct/tinkering/modifiers/a.json", "data/tconstruct/tinkering/tags/modifiers/abilities/a.json", "data/tconstruct/tinkering/materials/definition/a.json", "data/tconstruct/tinkering/tags/modifiers/../escape.json"];
        merge_pack(&mut raw, paths.iter().map(|s| s.to_string()), |_| Some(b"base".to_vec())).unwrap();
        merge_pack(&mut raw, paths.iter().map(|s| s.to_string()), |_| Some(b"higher".to_vec())).unwrap();
        assert_eq!(raw.0.len(), 1); assert_eq!(raw.1.len(), 1);
        assert_eq!(raw.0["tconstruct:tinkering/modifiers/a"], [b"base".to_vec(), b"higher".to_vec()]);
        assert_eq!(highest_definition(&raw, "tconstruct:tinkering/modifiers/a"), Some(&b"higher"[..]));
        assert_eq!(highest_definition(&raw, "tconstruct:tinkering/modifiers/missing"), None);
        assert_eq!(raw.1["tconstruct:tinkering/tags/modifiers/abilities/a"], [b"base".to_vec(), b"higher".to_vec()]);
        assert!(merge_pack(&mut raw, [paths[0].to_owned()], |_| None).is_err());
    }
    #[test]
    fn requires_registry_and_preserves_tag_operations_and_order() {
        let mut raw = Raw::default();
        raw.1.insert("tconstruct:tinkering/tags/modifiers/root".into(), vec![br##"{"values":["tconstruct:a","#tconstruct:child",{"id":"tconstruct:missing","required":false}]}"##.to_vec(), br##"{"values":["tconstruct:c"],"remove":["tconstruct:a"]}"##.to_vec()]);
        raw.1.insert("tconstruct:tinkering/tags/modifiers/child".into(), vec![br#"{"values":["tconstruct:b"]}"#.to_vec()]);
        let ids = ["tconstruct:a", "tconstruct:b", "tconstruct:c"].into_iter().map(str::to_owned).collect();
        assert!(resolve_tag(&raw, "tconstruct:root", None).is_err());
        assert_eq!(resolve_tag(&raw, "tconstruct:root", Some(&ids)).unwrap(), ["tconstruct:b", "tconstruct:c"]);
        raw.1.get_mut("tconstruct:tinkering/tags/modifiers/root").unwrap().push(br#"{"replace":true,"values":["tconstruct:a"]}"#.to_vec());
        assert_eq!(resolve_tag(&raw, "tconstruct:root", Some(&ids)).unwrap(), ["tconstruct:a"]);
    }
    #[test]
    fn required_missing_and_cycles_are_errors_not_empty_membership() {
        let mut raw = Raw::default(); let ids = HashSet::new();
        raw.1.insert("x:tinkering/tags/modifiers/root".into(), vec![br#"{"values":["x:absent"]}"#.to_vec()]);
        assert!(resolve_tag(&raw, "x:root", Some(&ids)).is_err());
        raw.1.insert("x:tinkering/tags/modifiers/root".into(), vec![br##"{"values":["#x:loop"]}"##.to_vec()]);
        raw.1.insert("x:tinkering/tags/modifiers/loop".into(), vec![br##"{"values":[{"id":"#x:root","required":false}]}"##.to_vec()]);
        assert!(resolve_tag(&raw, "x:root", Some(&ids)).is_err());
    }
    #[test]
    fn optional_nested_missing_is_skipped_but_required_is_not() {
        let mut raw = Raw::default();
        let ids: HashSet<String> = ["x:a".to_owned()].into_iter().collect();
        let key = "x:tinkering/tags/modifiers/root";
        raw.1.insert(key.into(), vec![br##"{"values":["x:a",{"id":"#x:absent","required":false}],"remove":["#x:missing"]}"##.to_vec()]);
        assert_eq!(resolve_tag(&raw, "x:root", Some(&ids)).unwrap(), ["x:a"]);
        raw.1.insert(key.into(), vec![br##"{"values":["#x:absent"]}"##.to_vec()]);
        assert!(resolve_tag(&raw, "x:root", Some(&ids)).is_err());
    }

    #[test]
    fn fresh_reload_does_not_retain_removed_pack_resources() {
        let path = "data/x/tinkering/modifiers/one.json";
        let mut before = Raw::default();
        merge_pack(&mut before, [path.to_owned()], |_| Some(b"old".to_vec())).unwrap();
        let mut after = Raw::default();
        merge_pack(&mut after, Vec::<String>::new(), |_| None).unwrap();
        assert!(highest_definition(&before, "x:tinkering/modifiers/one").is_some());
        assert!(highest_definition(&after, "x:tinkering/modifiers/one").is_none());
    }

}
