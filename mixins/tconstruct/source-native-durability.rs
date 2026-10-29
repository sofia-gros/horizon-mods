// Minecraft-side recycling uses the prepared Horizon 26.3 item registry.
// Unknown items and stack-level max-damage overrides are not guessed.
use std::collections::HashMap;

pub(super) fn max_damage(item: &str, values: &HashMap<String, u32>, has_override: bool) -> Option<i32> {
    if has_override || !item.starts_with("minecraft:") { return None; }
    values.get(item).copied().filter(|value| *value > 0 && *value <= i32::MAX as u32).map(|value| value as i32)
}
