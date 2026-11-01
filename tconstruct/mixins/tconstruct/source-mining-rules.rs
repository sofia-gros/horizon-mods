// Shared per-stack effective-block recognition and conditional calculations.
use horizon_world::nbt::Tag;
use super::tconstruct_source_mining_math::{Conditional, conditional_speed};

pub(super) fn effective_speed(tool: &Tag, block: &str, in_tag: impl Fn(&str, &str) -> bool) -> Option<f32> {
    let Some(Tag::List(rules)) = tool.get("rules") else { return None; };
    for rule in rules {
        let Some(Tag::String(blocks)) = rule.get("blocks") else { continue; };
        let Some(Tag::Float(value)) = rule.get("speed") else { continue; };
        let blocks = blocks.to_string_lossy();
        if blocks.strip_prefix('#').map_or_else(|| blocks == block, |tag| in_tag(tag, block)) {
            return Some(f32::from_bits(*value));
        }
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
