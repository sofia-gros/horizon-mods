// Adapt only the two pinned acquisition recipes in the generated source layer.
// The native 1.20.5+ rename does not create a second item or rewrite saves.
use serde_json::Value;
use std::path::Path;

pub const MATERIAL: &str = "tools/materials/turtle_scute";
pub const RECYCLING: &str = "tables/recycling/turtle_helmet";

pub fn adapt_recipe(id: &str, recipe: &mut Value) -> bool {
    let pointer = match id {
        MATERIAL if recipe["type"] == "tconstruct:material" => "/ingredient/item",
        RECYCLING if recipe["type"] == "tconstruct:part_builder_recycling" => "/results/tconstruct:scale/item",
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
    for id in [MATERIAL, RECYCLING] {
        let path = original.join(format!("data/tconstruct/recipes/{id}.json"));
        let mut recipe: Value = serde_json::from_slice(&std::fs::read(&path)?)?;
        if adapt_recipe(id, &mut recipe) {
            std::fs::write(path, serde_json::to_vec_pretty(&recipe)?)?;
        }
    }
    Ok(())
}
