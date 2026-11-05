// Adapt only pinned scute recipes in the generated source layer.
// The native 1.20.5+ rename does not create a second item or rewrite saves.
use serde_json::Value;
use std::path::Path;

pub const MATERIAL: &str = "tools/materials/turtle_scute";
pub const RECYCLING: &str = "tables/recycling/turtle_helmet";
pub const TURTLE_SHELL: &str = "tools/modifiers/defense/turtle_shell";

pub fn adapt_recipe(id: &str, recipe: &mut Value) -> bool {
    let pointer = match id {
        MATERIAL if recipe["type"] == "tconstruct:material" => "/ingredient/item",
        RECYCLING if recipe["type"] == "tconstruct:part_builder_recycling" => "/results/tconstruct:scale/item",
        TURTLE_SHELL if &*recipe == &serde_json::json!({
            "type":"tconstruct:incremental_modifier","allow_crystal":true,"amount_per_item":1,
            "input":{"item":"minecraft:scute"},"needed_per_level":5,"result":"tconstruct:turtle_shell",
            "slots":{"defense":1},"tools":{"tag":"tconstruct:modifiable/armor"}
        }) => "/input/item",
        _ => return false,
    };
    let Some(item) = recipe.pointer_mut(pointer) else { return false; };
    if item.as_str() != Some("minecraft:scute") { return false; }
    *item = Value::String("minecraft:turtle_scute".into());
    true
}

pub fn prepare(original: &Path, items: &Value) -> Result<(), Box<dyn std::error::Error>> {
    // Preserve hosts with a real legacy item and fail closed on unknown registries.
    let Some(items) = items.as_object() else { return Ok(()); };
    if !items.contains_key("minecraft:turtle_scute") || items.contains_key("minecraft:scute") {
        return Ok(());
    }
    for id in [MATERIAL, RECYCLING, TURTLE_SHELL] {
        let path = original.join(format!("data/tconstruct/recipes/{id}.json"));
        let mut recipe: Value = serde_json::from_slice(&std::fs::read(&path)?)?;
        if adapt_recipe(id, &mut recipe) {
            std::fs::write(path, serde_json::to_vec_pretty(&recipe)?)?;
        }
    }
    Ok(())
}


#[cfg(test)]
mod turtle_shell_tests {
    use super::*;
    fn recipe() -> Value {
        serde_json::json!({"type":"tconstruct:incremental_modifier","allow_crystal":true,"amount_per_item":1,
            "input":{"item":"minecraft:scute"},"needed_per_level":5,"result":"tconstruct:turtle_shell",
            "slots":{"defense":1},"tools":{"tag":"tconstruct:modifiable/armor"}})
    }
    #[test]
    fn rename_is_specific_and_idempotent() {
        let mut value = recipe();
        assert!(adapt_recipe(TURTLE_SHELL,&mut value));
        assert_eq!(value.pointer("/input/item").and_then(Value::as_str),Some("minecraft:turtle_scute"));
        assert!(!adapt_recipe(TURTLE_SHELL,&mut value));
        assert!(!adapt_recipe("custom:turtle_shell",&mut recipe()));
    }
    #[test]
    fn customized_source_recipe_is_not_rewritten() {
        let mut value = recipe();
        value["needed_per_level"] = serde_json::json!(4);
        assert!(!adapt_recipe(TURTLE_SHELL,&mut value));
        assert_eq!(value.pointer("/input/item").and_then(Value::as_str),Some("minecraft:scute"));
    }
}
