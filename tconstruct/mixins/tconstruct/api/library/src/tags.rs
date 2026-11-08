//! Data tags of the library's own registries (`TagLoader` over materials and modifiers): `data/<namespace>/tinkering/tags/<kind>/<name>.json`.
//!
//! A tag file is `{"replace": bool, "values": [...]}`: a value is an id, `#tag` for another tag of the same kind, or an object with `id`
//! and `required: false`. Members that do not exist are an error unless optional. Later files with the same id add to the earlier
//! ones unless `replace` is set.
use std::collections::{BTreeMap, HashSet};

use serde_json::Value;

use crate::resource::ResourceId;

/// Resolves tags to their members, in the order they were written, without repeats. `exists` says whether an id is a member of the
/// registry; the returned strings are the problems found, which the original logs.
pub fn load(files: &[(ResourceId, Value)], exists: &dyn Fn(&ResourceId) -> bool) -> (BTreeMap<ResourceId, Vec<ResourceId>>, Vec<String>) {
    let mut problems = Vec::new();
    let mut definitions: BTreeMap<ResourceId, Vec<Value>> = BTreeMap::new();
    for (id, json) in files {
        let values = json.get("values").and_then(Value::as_array).cloned().unwrap_or_default();
        if json.get("replace").and_then(Value::as_bool).unwrap_or(false) {
            definitions.insert(id.clone(), values);
        } else {
            definitions.entry(id.clone()).or_default().extend(values);
        }
    }
    let mut resolved = BTreeMap::new();
    for tag in definitions.keys() {
        let mut members = Vec::new();
        let mut seen = HashSet::new();
        resolve(tag, &definitions, exists, &mut members, &mut seen, &mut vec![tag.clone()], &mut problems);
        resolved.insert(tag.clone(), members);
    }
    (resolved, problems)
}

fn resolve(
    tag: &ResourceId,
    definitions: &BTreeMap<ResourceId, Vec<Value>>,
    exists: &dyn Fn(&ResourceId) -> bool,
    members: &mut Vec<ResourceId>,
    seen: &mut HashSet<ResourceId>,
    path: &mut Vec<ResourceId>,
    problems: &mut Vec<String>,
) {
    for value in &definitions[tag] {
        let (text, required) = match value {
            Value::String(text) => (text.as_str(), true),
            Value::Object(object) => (object.get("id").and_then(Value::as_str).unwrap_or(""), object.get("required").and_then(Value::as_bool).unwrap_or(true)),
            _ => ("", true),
        };
        if let Some(inner) = text.strip_prefix('#') {
            let Some(inner) = ResourceId::try_parse(inner) else {
                problems.push(format!("Tag {tag} has a bad reference '{text}'"));
                continue;
            };
            if path.contains(&inner) {
                problems.push(format!("Tag {tag} includes itself through {inner}"));
            } else if definitions.contains_key(&inner) {
                path.push(inner.clone());
                resolve(&inner, definitions, exists, members, seen, path, problems);
                path.pop();
            } else if required {
                problems.push(format!("Tag {tag} refers to the missing tag {inner}"));
            }
            continue;
        }
        let Some(id) = ResourceId::try_parse(text) else {
            problems.push(format!("Tag {tag} has a bad id '{text}'"));
            continue;
        };
        if exists(&id) {
            if seen.insert(id.clone()) {
                members.push(id);
            }
        } else if required {
            problems.push(format!("Tag {tag} lists {id}, which does not exist"));
        }
    }
}
