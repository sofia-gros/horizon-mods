use std::collections::{BTreeMap, HashSet};
pub(super) type Raw = BTreeMap<String, Vec<Vec<u8>>>;

pub(super) fn merge_pack(raw: &mut Raw, files: impl IntoIterator<Item=String>, read: impl Fn(&str)->Option<Vec<u8>>) -> Result<(), String> {
    for file in files {
        let Some(rest) = file.strip_prefix("data/") else { continue; };
        let Some((namespace, path)) = rest.split_once('/') else { continue; };
        let Some(path) = path.strip_prefix("tags/damage_type/").and_then(|p| p.strip_suffix(".json")) else { continue; };
        if namespace.is_empty() || path.split('/').any(|p| p.is_empty() || p == "." || p == "..") { continue; }
        let bytes = read(&file).ok_or_else(|| format!("unreadable damage tag {file}"))?;
        raw.entry(format!("{namespace}:{path}")).or_default().push(bytes);
    }
    Ok(())
}

/// Source-only damage IDs cannot occur in this host. Project concrete IDs
/// onto its actual catalog; required nested tags and cycles still fail closed.
pub(super) fn project(id: &str, raw: &Raw, registry: &HashSet<String>) -> Result<HashSet<String>, String> {
    fn canonical(id: &str) -> String { if id.contains(':') { id.to_owned() } else { format!("minecraft:{id}") } }
    fn visit(id: &str, raw: &Raw, registry: &HashSet<String>, visiting: &mut Vec<String>) -> Result<Vec<String>, String> {
        if visiting.iter().any(|entry| entry == id) { return Err(format!("cyclic damage tag {id}")); }
        let layers = raw.get(id).ok_or_else(|| format!("missing damage tag {id}"))?;
        if !layers.iter().any(|bytes| super::tconstruct_source_tags::parse_file(bytes).is_some()) {
            return Err(format!("invalid damage tag {id}"));
        }
        visiting.push(id.to_owned());
        let mut result = Vec::new();
        for entry in super::tconstruct_source_tags::merge_tag_files(layers) {
            let values = if let Some(tag) = entry.id.strip_prefix('#') {
                match visit(&canonical(tag), raw, registry, visiting) {
                    Ok(values) => values,
                    Err(error) if error.starts_with("cyclic damage tag ") => { visiting.pop(); return Err(error); },
                    Err(_) if entry.remove || !entry.required => continue,
                    Err(error) => { visiting.pop(); return Err(error); },
                }
            } else {
                let id = canonical(&entry.id);
                if registry.contains(&id) { vec![id] } else { Vec::new() }
            };
            for value in values {
                if entry.remove { result.retain(|old| old != &value); }
                else if !result.contains(&value) { result.push(value); }
            }
        }
        visiting.pop();
        Ok(result)
    }
    visit(&canonical(id), raw, registry, &mut Vec::new()).map(|ids| ids.into_iter().collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn damage_tags_project_unavailable_ids_and_keep_replace_remove_order() {
        let registry = ["minecraft:in_fire", "minecraft:lightning_bolt"].map(str::to_owned).into_iter().collect();
        let mut raw = Raw::from([
            ("minecraft:is_fire".into(), vec![br#"{"values":["minecraft:in_fire"]}"#.to_vec()]),
            ("tconstruct:protection/fire".into(), vec![br##"{"values":["#minecraft:is_fire","minecraft:lightning_bolt","tconstruct:shock"]}"##.to_vec()]),
        ]);
        assert_eq!(project("tconstruct:protection/fire", &raw, &registry).unwrap().len(), 2);
        raw.get_mut("tconstruct:protection/fire").unwrap().push(br#"{"values":[],"remove":["minecraft:in_fire"]}"#.to_vec());
        assert_eq!(project("tconstruct:protection/fire", &raw, &registry).unwrap(), HashSet::from(["minecraft:lightning_bolt".into()]));
        raw.get_mut("tconstruct:protection/fire").unwrap().push(br#"{"replace":true,"values":[]}"#.to_vec());
        assert!(project("tconstruct:protection/fire", &raw, &registry).unwrap().is_empty());
    }
    #[test]
    fn unreadable_resources_and_required_nested_tags_are_not_guessed() {
        let mut raw = Raw::new();
        assert!(merge_pack(&mut raw, ["data/tconstruct/tags/damage_type/protection/fire.json".into()], |_| None).is_err());
        raw.insert("tconstruct:protection/fire".into(), vec![br##"{"values":["#test:missing"]}"##.to_vec()]);
        assert!(project("tconstruct:protection/fire", &raw, &HashSet::new()).is_err());
        raw.insert("test:missing".into(), vec![br##"{"values":["#tconstruct:protection/fire"]}"##.to_vec()]);
        assert!(project("tconstruct:protection/fire", &raw, &HashSet::new()).unwrap_err().starts_with("cyclic damage tag "));
    }
}
