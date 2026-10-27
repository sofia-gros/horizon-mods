//! State transport fixture. This does not attach tool stats or durability hooks.
use crate::{Catalog, ToolState};

pub const FEEDBACK: &str = "Queued a state specimen. Use /tconstruct_tool activate to enable mining when the server mod is installed.";

pub fn command(catalog: &Catalog, args: &str) -> Result<String, String> {
    let words: Vec<_> = args.split_whitespace().collect();
    let (parts, damage) = match words.as_slice() {
        ["specimen", head, handle, binding] => ([*head, *handle, *binding], 0),
        ["specimen", head, handle, binding, damage] => (
            [*head, *handle, *binding],
            damage
                .parse::<u32>()
                .map_err(|_| "damage must be an unsigned integer")?,
        ),
        _ => {
            return Err(
                "usage: /tconstruct specimen <head> <handle> <binding> [core-damage]".into(),
            );
        }
    };
    let mut state = ToolState::pickaxe(parts);
    catalog.evaluate(&state)?;
    state.damage_points(catalog, damage)?;
    // SNBT single quotes keep the complete JSON inside the host's 256-character
    // command limit. Accepted materials come only from the embedded catalog.
    let payload = state.to_json()?.replace('\\', "\\\\").replace('\'', "\\'");
    Ok(format!(
        "give @s wooden_pickaxe[custom_name=\"TConstruct state specimen\",custom_data={{tconstruct:{{state_json:'{payload}'}}}}] 1"
    ))
}
