use crate::crafting::Ingredient;
use horizon_items::items::Items;
// Explicit opt-in for the six source decorative stonecutting ingredients.
// Resolve against the Items supplied to this table load, never a fixed tag list.
fn tconstruct_stonecutting_ingredient(v: &serde_json::Value, items: &Items) -> std::ops::ControlFlow<Ingredient> {
    use std::ops::ControlFlow::{Break, Continue};
    fn valid_id(id: &str) -> bool {
        id.split_once(':').is_some_and(|(namespace, path)| {
            !namespace.is_empty() && !path.is_empty()
                && namespace.bytes().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || b"_.-".contains(&c))
                && path.bytes().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || b"_./-".contains(&c))
        })
    }
    fn resolve(v: &serde_json::Value, items: &Items, depth: u8) -> Option<Vec<String>> {
        if depth > 8 { return None; }
        if let Some(children) = v.as_array() {
            if children.is_empty() || children.len() > 16 { return None; }
            let mut result = Vec::new();
            let mut seen = std::collections::HashSet::new();
            for child in children {
                for id in resolve(child, items, depth + 1)? {
                    if seen.insert(id.clone()) { result.push(id); }
                }
            }
            return Some(result);
        }
        let object = v.as_object()?;
        if object.len() == 1 {
            if let Some(id) = object.get("item") {
                let id = id.as_str()?;
                return (valid_id(id) && items.items.contains_key(id)).then(|| vec![id.to_owned()]);
            }
            let tag = object.get("tag")?.as_str()?;
            if !valid_id(tag) { return None; }
            // A missing/empty tag contributes no members, including in a union.
            let mut result = Vec::new();
            let mut seen = std::collections::HashSet::new();
            for id in items.item_tag_order.get(tag).into_iter().flatten() {
                if !valid_id(id) || !items.items.contains_key(id.as_str()) { return None; }
                if seen.insert(id) { result.push(id.clone()); }
            }
            return Some(result);
        }
        if object.len() != 3 || object.get("type")?.as_str()? != "forge:difference" { return None; }
        let mut base = resolve(object.get("base")?, items, depth + 1)?;
        let excluded: std::collections::HashSet<_> = resolve(object.get("subtracted")?, items, depth + 1)?.into_iter().collect();
        base.retain(|id| !excluded.contains(id));
        Some(base)
    }
    if v.get("type").and_then(serde_json::Value::as_str) != Some("tconstruct:stonecutting_ingredient") {
        return Continue(());
    }
    let resolved = v.as_object().filter(|object| object.len() == 2)
        .and_then(|object| object.get("value")).and_then(|value| resolve(value, items, 0));
    // The station parser's normal failure representation is an empty ingredient.
    // Do not fall through and accidentally ignore an unsupported subtraction.
    Break(Ingredient::of(resolved.unwrap_or_default()))
}

pub fn parse(value: &serde_json::Value, items: &Items) -> Option<Ingredient> { match tconstruct_stonecutting_ingredient(value,items) { std::ops::ControlFlow::Break(v) => Some(v), std::ops::ControlFlow::Continue(()) => None } }
