// Shared per-stack effective-block recognition and conditional calculations.
use horizon_world::nbt::Tag;
use super::tconstruct_source_mining_math::{Conditional, conditional_speed};

/// Haste on a non-harvest chestplate applies the source player mining-speed multiplier.
pub(super) fn haste_armor_multiplier(custom: &Tag, damage: u32) -> f32 {
    let Some(Tag::Int(max)) = custom.get("tic_max_damage") else { return 1.0; };
    if *max <= 0 || damage >= *max as u32 { return 1.0; }
    let amount = match custom.get("tic_upgrades").and_then(|upgrades| upgrades.get("tconstruct:haste")) {
        Some(Tag::Int(amount)) => (*amount).max(0), _ => 0,
    };
    1.0 + 0.1 * amount as f32 / 45.0
}

#[cfg(test)]
mod haste_tests {
    use super::*;

    #[test]
    fn partial_haste_only_applies_on_unbroken_armor() {
        let custom = Tag::compound([
            ("tic_max_damage", Tag::Int(100)),
            ("tic_upgrades", Tag::compound([("tconstruct:haste", Tag::Int(18))])),
        ]);
        assert!((haste_armor_multiplier(&custom, 99) - 1.04).abs() < 0.00001);
        assert_eq!(haste_armor_multiplier(&custom, 100), 1.0);
    }
}

fn matches(rule: &Tag, block: &str, in_tag: &impl Fn(&str, &str) -> bool) -> bool {
    let Some(Tag::String(blocks)) = rule.get("blocks") else { return false; };
    let blocks = blocks.to_string_lossy();
    blocks.strip_prefix('#').map_or_else(|| blocks == block, |tag| in_tag(tag, block))
}

pub(super) fn harvestable(tool: &Tag, block: &str, in_tag: impl Fn(&str, &str) -> bool) -> Option<bool> {
    let Some(Tag::List(rules)) = tool.get("rules") else { return None; };
    for rule in rules {
        if !matches(rule, block, &in_tag) { continue; }
        if let Some(Tag::Byte(correct)) = rule.get("correct_for_drops") { return Some(*correct != 0); }
    }
    Some(false)
}

pub(super) fn effective_speed(tool: &Tag, block: &str, in_tag: impl Fn(&str, &str) -> bool) -> Option<f32> {
    let Some(Tag::List(rules)) = tool.get("rules") else { return None; };
    for rule in rules {
        if !matches(rule, block, &in_tag) { continue; }
        if matches!(rule.get("correct_for_drops"), Some(Tag::Byte(0))) { return None; }
        if let Some(Tag::Float(value)) = rule.get("speed") { return Some(f32::from_bits(*value)); }
    }
    None
}

pub(super) fn conditional(custom: &Tag, damage: u32, speed: f32, potion_multiplier: f32) -> Option<f32> {
    let Some(Tag::Int(max)) = custom.get("tic_max_damage") else { return None; };
    if *max <= 0 || damage >= *max as u32 { return None; }
    let Some(Tag::List(modifiers)) = custom.get("tic_mining_traits") else { return None; };
    let multiplier = |key| match custom.get(key) { Some(Tag::Float(bits)) => f32::from_bits(*bits), _ => 1.0 };
    let mut ordered = Vec::new();
    for modifier in modifiers {
        let (Some(Tag::String(id)), Some(Tag::Int(level))) = (modifier.get("id"), modifier.get("level")) else { continue; };
        let effect = match id.to_string_lossy().as_ref() {
            "tconstruct:jagged" => Conditional::Jagged(*level),
            "tconstruct:stonebound" => Conditional::Stonebound(*level),
            "tconstruct:maintained" => Conditional::Maintained(*level),
            _ => continue,
        };
        ordered.push(effect);
    }
    Some(conditional_speed(speed, damage, *max as u32, multiplier("tic_durability_all"), multiplier("tic_mining_all") * potion_multiplier, &ordered))
}
