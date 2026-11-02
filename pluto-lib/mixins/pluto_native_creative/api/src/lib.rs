//! Prepare component-bearing Creative tabs for Horizon's native page UI.
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error(pub String);
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for Error {}

/// A stack in a tab. Search metadata is supplied by its owner.
#[derive(Debug, Clone)]
pub struct Entry {
    pub snbt: String,
    pub searchable: bool,
    pub permissions_only: bool,
    pub search_lines: Vec<String>,
    pub search_tags: Vec<String>,
}
#[derive(Debug, Clone)]
pub struct Tab {
    pub id: String,
    pub title: String,
    pub icon: String,
    pub items: Vec<Entry>,
}
fn part(s: &str) -> bool {
    !s.is_empty()
        && s != "."
        && s != ".."
        && s.bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"_.-".contains(&b))
}
fn valid_id(id: &str) -> bool {
    id.split_once(':')
        .is_some_and(|(ns, path)| part(ns) && path.split('/').all(part))
}
fn err(s: impl Into<String>) -> Error {
    Error(s.into())
}

/// Return a new document. `identity` validates a stack and returns a canonical
/// item-and-components key, ignoring count. It must agree with the target host.
/// Existing entries win on equal identity; conflicting permission flags fail.
pub fn register_tabs<F>(inventory: &Value, groups: &[Tab], identity: F) -> Result<Value, Error>
where
    F: Fn(&str) -> Result<String, String>,
{
    let mut out = inventory.clone();
    let tabs = out
        .get_mut("tabs")
        .and_then(Value::as_array_mut)
        .ok_or_else(|| err("missing tabs array"))?;
    let mut ids = BTreeSet::new();
    let mut occupied = BTreeSet::new();
    let mut search = None;
    for (i, tab) in tabs.iter().enumerate() {
        let obj = tab.as_object().ok_or_else(|| err("invalid existing tab"))?;
        let id = obj
            .get("id")
            .and_then(Value::as_str)
            .filter(|id| valid_id(id))
            .ok_or_else(|| err("invalid existing tab ID"))?;
        if !ids.insert(id.to_owned()) {
            return Err(err("duplicate existing tab ID"));
        }
        if obj.get("type").and_then(Value::as_str) == Some("SEARCH") && search.replace(i).is_some()
        {
            return Err(err("multiple SEARCH tabs"));
        }
        let row = obj
            .get("row")
            .and_then(Value::as_str)
            .filter(|row| matches!(*row, "TOP" | "BOTTOM"))
            .ok_or_else(|| err("invalid existing tab row"))?;
        let col = obj
            .get("column")
            .and_then(Value::as_u64)
            .filter(|col| *col < 7)
            .ok_or_else(|| err("invalid existing tab column"))?;
        let page = match obj.get("page") {
            Some(value) => value
                .as_u64()
                .filter(|page| *page <= u32::MAX as u64)
                .ok_or_else(|| err("invalid existing tab page"))?,
            None => 0,
        };
        if col < 5 && !occupied.insert((page, row.to_owned(), col)) {
            return Err(err("overlapping existing tab position"));
        }
    }
    let mut next_slot = 0u64;
    let mut candidates = Vec::new();
    for tab in groups {
        if !valid_id(&tab.id) || tab.title.is_empty() || !ids.insert(tab.id.clone()) {
            return Err(err("invalid or duplicate tab ID/title"));
        }
        identity(&tab.icon).map_err(err)?;
        let mut only = Vec::new();
        for (i, entry) in tab.items.iter().enumerate() {
            let key = identity(&entry.snbt).map_err(err)?;
            if entry.permissions_only {
                only.push(i);
            }
            if entry.searchable {
                candidates.push((key, entry));
            }
        }
        let (page, row, col) = loop {
            let page = 1 + next_slot / 10;
            if page > u32::MAX as u64 {
                return Err(err("too many pages"));
            }
            let slot = next_slot % 10;
            next_slot += 1;
            let row = if slot < 5 { "TOP" } else { "BOTTOM" };
            let col = slot % 5;
            if occupied.insert((page, row.to_owned(), col)) {
                break (page, row, col);
            }
        };
        tabs.push(json!({"id":tab.id,"type":"CATEGORY","row":row,"column":col,"page":page,
            "title":tab.title,"icon":tab.icon,"visible":true,
            "items":tab.items.iter().map(|item| &item.snbt).collect::<Vec<_>>(),"permissions_only":only}));
    }
    if candidates.is_empty() {
        return Ok(out);
    }
    let index = search.ok_or_else(|| err("missing SEARCH tab"))?;
    let obj = tabs[index]
        .as_object_mut()
        .ok_or_else(|| err("invalid SEARCH tab"))?;
    let mut items = obj
        .get("items")
        .and_then(Value::as_array)
        .cloned()
        .ok_or_else(|| err("invalid search items"))?;
    let mut lines = obj
        .get("lines")
        .map(|v| {
            v.as_array()
                .cloned()
                .ok_or_else(|| err("invalid search lines"))
        })
        .transpose()?
        .unwrap_or_default();
    let mut tags = obj
        .get("tags")
        .map(|v| {
            v.as_array()
                .cloned()
                .ok_or_else(|| err("invalid search tags"))
        })
        .transpose()?
        .unwrap_or_default();
    if lines.len() > items.len() || tags.len() > items.len() {
        return Err(err("search metadata exceeds item count"));
    }
    let mut only = BTreeSet::new();
    if let Some(value) = obj.get("permissions_only") {
        for v in value
            .as_array()
            .ok_or_else(|| err("invalid search permissions"))?
        {
            let i = v
                .as_u64()
                .ok_or_else(|| err("invalid search permission index"))?;
            if i >= items.len() as u64 || !only.insert(i) {
                return Err(err("invalid search permission index"));
            }
        }
    }
    let mut known = BTreeMap::new();
    for (i, value) in items.iter().enumerate() {
        let stack = value.as_str().ok_or_else(|| err("invalid search stack"))?;
        let key = identity(stack).map_err(err)?;
        let restricted = only.contains(&(i as u64));
        if let Some(previous) = known.insert(key, restricted) {
            if previous != restricted {
                return Err(err("conflicting existing search permissions"));
            }
        }
    }
    lines.resize(items.len(), json!([]));
    tags.resize(items.len(), json!([]));
    for (key, entry) in candidates {
        if let Some(restricted) = known.get(&key) {
            if *restricted != entry.permissions_only {
                return Err(err("conflicting search permissions for stack"));
            }
            continue;
        }
        if entry.permissions_only {
            only.insert(items.len() as u64);
        }
        known.insert(key, entry.permissions_only);
        items.push(json!(entry.snbt));
        lines.push(json!(entry.search_lines));
        tags.push(json!(entry.search_tags));
    }
    obj.insert("items".into(), items.into());
    obj.insert("lines".into(), lines.into());
    obj.insert("tags".into(), tags.into());
    obj.insert(
        "permissions_only".into(),
        only.into_iter().map(Value::from).collect::<Vec<_>>().into(),
    );
    Ok(out)
}
