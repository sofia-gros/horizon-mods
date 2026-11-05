//! The native vanilla Chain -> Iron Chain rename in the generated source layer.
use serde_json::Value;
use std::path::Path;

pub const RECIPE: &str = "smeltery/melting/metal/iron/chain";

pub fn adapt_recipe(recipe: &mut Value) -> bool {
    if recipe["type"] != "tconstruct:melting" { return false; }
    let Some(item) = recipe.pointer_mut("/ingredient/item") else { return false; };
    if item.as_str() != Some("minecraft:chain") { return false; }
    *item = Value::String("minecraft:iron_chain".into());
    true
}

pub fn prepare(original: &Path, items: &Value) -> Result<(), Box<dyn std::error::Error>> {
    let Some(items) = items.as_object() else { return Ok(()); };
    if !items.contains_key("minecraft:iron_chain") || items.contains_key("minecraft:chain") {
        return Ok(());
    }
    // Never scan or rewrite the player's enabled world datapacks.
    let path = original.join(format!("data/tconstruct/recipes/{RECIPE}.json"));
    let mut recipe: Value = serde_json::from_slice(&std::fs::read(&path)?)?;
    if adapt_recipe(&mut recipe) {
        std::fs::write(path, serde_json::to_vec_pretty(&recipe)?)?;
    }
    Ok(())
}
