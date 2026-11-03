// Minecraft-side recycling uses the prepared Horizon 26.3 item registry.
// Unknown items and unsupported stack-level overrides are not guessed.
use std::collections::HashMap;
use horizon_world::nbt::Tag;

pub(super) fn max_damage(item: &str, values: &HashMap<String, u32>, override_damage: Option<&Tag>) -> Option<i32> {
    if !item.starts_with("minecraft:") { return None; }
    let registered = values.get(item).copied().filter(|value| *value > 0 && *value <= i32::MAX as u32)? as i32;
    match override_damage {
        None => Some(registered),
        Some(Tag::Int(value)) if *value > 0 => Some(*value),
        _ => None,
    }
}
