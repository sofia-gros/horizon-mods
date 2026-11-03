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

/// Recipes with no conditions, including an empty list, need no snapshot.
/// A nonempty or malformed list must still go through the fail-closed gate.
pub(super) fn unconditional_recipe(bytes: &[u8]) -> Option<Vec<u8>> {
    let mut value: serde_json::Value = serde_json::from_slice(bytes).ok()?;
    match value.get("conditions") {
        None => Some(bytes.to_vec()),
        Some(serde_json::Value::Array(conditions)) if conditions.is_empty() => {
            value.as_object_mut()?.remove("conditions");
            serde_json::to_vec(&value).ok()
        },
        _ => None,
    }
}

/// Gate an ordinary recipe using the same snapshot as the Forge wrapper.
/// Once accepted, remove the already-evaluated list: downstream parsers may
/// lack item/mod registry context and must not evaluate it a second time.
pub(super) fn resolved_recipe_conditions(bytes: &[u8], registries: &HashMap<String, Tags>, config: &HashMap<String, bool>, items: &HashSet<String>, mods: Option<&ModPresence>) -> Option<Vec<u8>> {
    let mut value: serde_json::Value = serde_json::from_slice(bytes).ok()?;
    let Some(raw) = value.get("conditions") else { return Some(bytes.to_vec()); };
    let conditions = raw.as_array()?;
    if conditional_conditions_with_mods(conditions, registries, config, items, mods) != Some(true) { return None; }
    value.as_object_mut()?.remove("conditions");
    serde_json::to_vec(&value).ok()
}

#[cfg(test)]
mod snapshot_tests {
    use super::*;
    #[test]
    fn accepted_item_exists_is_not_rechecked_by_legacy_consumers() {
        let tags = HashMap::new(); let config = HashMap::new();
        let items = ["example:hammer".to_owned()].into_iter().collect();
        let recipe = br#"{"type":"tconstruct:melting","conditions":[{"type":"forge:item_exists","item":"example:hammer"}],"result":{"fluid":"example:metal"}}"#;
        let result = resolved_recipe_conditions(recipe, &tags, &config, &items, None).unwrap();
        let parsed: serde_json::Value = serde_json::from_slice(&result).unwrap();
        assert!(parsed.get("conditions").is_none());
        assert_eq!(parsed["result"]["fluid"], "example:metal");
        assert!(resolved_recipe_conditions(recipe, &tags, &config, &HashSet::new(), None).is_none());
    }
    #[test]
    fn empty_conditions_need_no_registry_or_config_snapshot() {
        let recipe = br#"{"type":"example:recipe","conditions":[]}"#;
        let result = unconditional_recipe(recipe).unwrap();
        let parsed: serde_json::Value = serde_json::from_slice(&result).unwrap();
        assert!(parsed.get("conditions").is_none());
        assert_eq!(parsed["type"], "example:recipe");
        assert!(unconditional_recipe(br#"{"conditions":[{"type":"forge:true"}]}"#).is_none());
        assert!(unconditional_recipe(br#"{"conditions":{}}"#).is_none());
    }
    #[test]
    fn selected_wrapper_recipe_still_gates_its_own_conditions() {
        let tags = HashMap::new(); let config = HashMap::new();
        let items = ["example:hammer".to_owned()].into_iter().collect();
        let wrapper = br#"{"type":"forge:conditional","recipes":[{"conditions":[{"type":"forge:true"}],"recipe":{"type":"example:melting","conditions":[{"type":"forge:item_exists","item":"example:hammer"}]}}]}"#;
        let selected = conditional_recipe(wrapper, &tags, &config, &items, None).unwrap();
        assert!(resolved_recipe_conditions(&selected, &tags, &config, &HashSet::new(), None).is_none());
        let result = resolved_recipe_conditions(&selected, &tags, &config, &items, None).unwrap();
        let parsed: serde_json::Value = serde_json::from_slice(&result).unwrap();
        assert_eq!(parsed["type"], "example:melting");
        assert!(parsed.get("conditions").is_none());
        let unconditional = br#"{ "type": "example:recipe" }"#;
        assert_eq!(resolved_recipe_conditions(unconditional, &tags, &config, &items, None).unwrap(), unconditional);
    }
    #[test]
    fn compound_mod_and_unknown_conditions_are_gated_before_removal() {
        let tags = HashMap::new(); let config = HashMap::new(); let items = HashSet::new();
        let mods = ModPresence { states: [("example".to_owned(), Some(true))].into_iter().collect(), complete: true };
        let recipe = br#"{"type":"example:recipe","conditions":[{"type":"forge:and","values":[{"type":"forge:mod_loaded","modid":"example"},{"type":"forge:not","value":{"type":"forge:false"}}]}]}"#;
        assert!(resolved_recipe_conditions(recipe, &tags, &config, &items, Some(&mods)).is_some());
        assert!(resolved_recipe_conditions(recipe, &tags, &config, &items, None).is_none());
        let unknown = br#"{"type":"example:recipe","conditions":[{"type":"example:unknown"}]}"#;
        assert!(resolved_recipe_conditions(unknown, &tags, &config, &items, Some(&mods)).is_none());
        assert!(resolved_recipe_conditions(br#"{"conditions":{}}"#, &tags, &config, &items, Some(&mods)).is_none());
    }
}
