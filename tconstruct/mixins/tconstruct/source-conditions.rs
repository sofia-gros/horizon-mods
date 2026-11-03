// Shared Forge condition resolution, used by host and server station contexts.
use std::collections::{HashMap, HashSet};
type Tags = HashMap<String, HashSet<String>>;
pub(super) struct ModPresence { pub states: HashMap<String, Option<bool>>, pub complete: bool }

// Resolve conditional recipes in source order. An unknown condition stops
// selection: falling through could select a different recipe than Forge.
fn conditional_test(value: &serde_json::Value, registries: &HashMap<String, Tags>, config: &HashMap<String, bool>, items: &HashSet<String>, mods: Option<&ModPresence>) -> Option<bool> {
    match value.get("type")?.as_str()? {
        "forge:true" => Some(true),
        "forge:false" => Some(false),
        "forge:item_exists" => Some(items.contains(value.get("item")?.as_str()?)),
        "forge:mod_loaded" => {
            let id = value.get("modid")?.as_str()?.trim();
            if id.is_empty() { return None; }
            let mods = mods?;
            mods.states.get(id).copied().unwrap_or_else(|| mods.complete.then_some(false))
        }
        "forge:not" => Some(!conditional_test(value.get("value")?, registries, config, items, mods)?),
        kind @ ("forge:and" | "forge:or") => {
            let values = value.get("values")?.as_array()?;
            if values.is_empty() { return None; }
            let mut unknown = false;
            for child in values {
                match conditional_test(child, registries, config, items, mods) {
                    Some(result) if (kind == "forge:and" && !result) || (kind == "forge:or" && result) => return Some(result),
                    Some(_) => {}, None => unknown = true,
                }
            }
            if unknown { None } else { Some(kind == "forge:and") }
        }
        _ => super::tconstruct_tx::source_conditions_match(std::slice::from_ref(value), registries, config),
    }
}

pub(super) fn conditional_conditions(values: &[serde_json::Value], registries: &HashMap<String, Tags>, config: &HashMap<String, bool>, items: &HashSet<String>) -> Option<bool> {
    conditional_conditions_with_mods(values, registries, config, items, None)
}
pub(super) fn conditional_conditions_with_mods(values: &[serde_json::Value], registries: &HashMap<String, Tags>, config: &HashMap<String, bool>, items: &HashSet<String>, mods: Option<&ModPresence>) -> Option<bool> {
    for value in values { if !conditional_test(value, registries, config, items, mods)? { return Some(false); } }
    Some(true)
}

pub(super) fn conditional_recipe(bytes: &[u8], registries: &HashMap<String, Tags>, config: &HashMap<String, bool>, items: &HashSet<String>, mods: Option<&ModPresence>) -> Option<Vec<u8>> {
    // Most entries are ordinary recipes; keep their exact bytes untouched.
    if !bytes.windows(b"forge:conditional".len()).any(|part| part == b"forge:conditional") { return Some(bytes.to_vec()); }
    let mut value: serde_json::Value = serde_json::from_slice(bytes).ok()?;
    for _ in 0..16 {
        if value.get("type").and_then(serde_json::Value::as_str) != Some("forge:conditional") {
            return serde_json::to_vec(&value).ok();
        }
        match value.get("conditions") {
            None => {},
            Some(serde_json::Value::Array(conditions)) if conditional_conditions_with_mods(conditions, registries, config, items, mods)? => {},
            _ => return None,
        }
        let branches = value.get("recipes")?.as_array()?;
        let mut selected = None;
        for branch in branches {
            let conditions = branch.get("conditions")?.as_array()?;
            let mut matches = true;
            for condition in conditions {
                if !conditional_test(condition, registries, config, items, mods)? { matches = false; break; }
            }
            if matches { selected = Some(branch.get("recipe")?.clone()); break; }
        }
        value = selected?;
    }
    None
}

