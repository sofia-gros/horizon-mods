// Shared per-stack effective-block recognition and conditional calculations.
use horizon_world::nbt::Tag;
use super::tconstruct_source_mining_math::{Conditional, conditional_speed_for};

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

// Evaluate the actual Tool with PlutoLib. A missing or invalid component leaves
// host fallback decisions to the caller.
fn outcome(tool: &Tag, block: &str, in_tag: impl Fn(&str, &str) -> bool) -> Option<super::tconstruct_tool_api::rules::Outcome> {
    let snbt = super::tconstruct_tool_api::canon_tag(tool);
    let parsed = super::tconstruct_tool_api::rules::parse_tool(Some(&snbt)).ok()??;
    Some(parsed.evaluate(|spec| spec.strip_prefix('#').map_or_else(|| spec == block, |tag| in_tag(tag, block))))
}

pub(super) fn harvestable(tool: &Tag, block: &str, in_tag: impl Fn(&str, &str) -> bool) -> Option<bool> {
    Some(outcome(tool, block, in_tag)?.correct_for_drops)
}

pub(super) fn evaluated_speed(tool: &Tag, block: &str, in_tag: impl Fn(&str, &str) -> bool) -> Option<Option<f32>> {
    let result = outcome(tool, block, &in_tag)?;
    let Some(speed_rule) = result.speed_rule else { return Some(None); };
    // TConstruct's tier policy rejects any matching explicit denial before
    // the selected speed, even if an earlier rule already allowed drops.
    let Tag::List(rules) = tool.get("rules")? else { return None; };
    for rule in rules.iter().take(speed_rule + 1) {
        if !matches!(rule.get("correct_for_drops"), Some(Tag::Byte(0))) { continue; }
        let Some(Tag::String(blocks)) = rule.get("blocks") else { continue; };
        let blocks = blocks.to_string_lossy();
        if blocks.strip_prefix('#').map_or_else(|| blocks == block, |tag| in_tag(tag, block)) { return Some(None); }
    }
    Some(Some(result.speed))
}

pub(super) fn effective_speed(tool: &Tag, block: &str, in_tag: impl Fn(&str, &str) -> bool) -> Option<f32> {
    evaluated_speed(tool, block, in_tag).flatten()
}

pub(super) fn conditional(custom: &Tag, damage: u32, speed: f32, potion_multiplier: f32, position: Option<(i32, i32)>, effective: bool, tool_not_required: bool) -> Option<f32> {
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
            "tconstruct:dwarven" => Conditional::Dwarven(*level),
            "tconstruct:crumbling" => Conditional::Crumbling(*level),
            _ => continue,
        };
        ordered.push(effect);
    }
    if !effective && !(tool_not_required && ordered.iter().any(|effect| matches!(effect, Conditional::Crumbling(level) if *level > 0))) { return None; }
    Some(conditional_speed_for(speed, damage, *max as u32, multiplier("tic_durability_all"), multiplier("tic_mining_all") * potion_multiplier, &ordered, position, effective, tool_not_required))
}

#[cfg(test)]
mod shared_rules_tests {
    use super::*;

    #[test]
    fn crumbling_respects_effectiveness_block_and_breakage() {
        let custom = Tag::compound([
            ("tic_max_damage", Tag::Int(100)),
            ("tic_mining_all", Tag::Float(1.5f32.to_bits())),
            ("tic_mining_traits", Tag::List(vec![Tag::compound([("id", Tag::String("tconstruct:crumbling".into())), ("level", Tag::Int(2))])])),
        ]);
        assert_eq!(conditional(&custom, 0, 1.0, 1.0, None, false, true), Some(4.0));
        assert_eq!(conditional(&custom, 0, 1.0, 1.0, None, false, false), None);
        let tool = Tag::compound([("rules", Tag::List(vec![Tag::compound([("blocks", Tag::String("minecraft:stone".into())), ("speed", Tag::Float(4.0f32.to_bits()))])]))]);
        let effective = evaluated_speed(&tool, "minecraft:dirt", |_, _| false).unwrap().is_some();
        assert!(!effective);
        assert_eq!(conditional(&custom, 0, 1.0, 1.0, None, effective, true), Some(4.0));
        assert_eq!(conditional(&custom, 100, 1.0, 1.0, None, false, true), None);
    }

    #[test]
    fn actual_tool_tier_precedes_speed_and_no_match_is_distinct() {
        let tool = Tag::compound([("rules", Tag::List(vec![
            Tag::compound([("blocks", Tag::String("#minecraft:incorrect_for_wooden_tool".into())), ("correct_for_drops", Tag::Byte(0))]),
            Tag::compound([("blocks", Tag::String("#minecraft:mineable/pickaxe".into())), ("speed", Tag::Float(2.0f32.to_bits())), ("correct_for_drops", Tag::Byte(1))]),
        ]))]);
        let matches = |tag: &str, block: &str| block == "minecraft:stone" && matches!(tag, "minecraft:incorrect_for_wooden_tool" | "minecraft:mineable/pickaxe");
        assert_eq!(effective_speed(&tool, "minecraft:stone", matches), None);
        assert_eq!(harvestable(&tool, "minecraft:stone", matches), Some(false));
        assert_eq!(effective_speed(&tool, "minecraft:dirt", matches), None);
        assert_eq!(harvestable(&tool, "minecraft:dirt", matches), Some(false));
        assert_eq!(evaluated_speed(&Tag::String("bad".into()), "minecraft:stone", matches), None);
        assert_eq!(evaluated_speed(&tool, "minecraft:dirt", matches), Some(None));
    }

    #[test]
    fn absent_and_invalid_tool_are_not_no_matching_rule() {
        assert!(super::super::tconstruct_tool_api::rules::parse_tool(None).unwrap().is_none());
        assert_eq!(evaluated_speed(&Tag::String("bad".into()), "minecraft:stone", |_, _| false), None);
        let empty = Tag::compound([("rules", Tag::List(Vec::new()))]);
        assert_eq!(evaluated_speed(&empty, "minecraft:stone", |_, _| false), Some(None));
    }

    #[test]
    fn denial_after_an_allow_still_blocks_tconstruct_speed() {
        let rule = |correct| Tag::compound([("blocks", Tag::String("minecraft:stone".into())), ("correct_for_drops", Tag::Byte(correct))]);
        let tool = Tag::compound([("rules", Tag::List(vec![rule(1), rule(0), Tag::compound([
            ("blocks", Tag::String("minecraft:stone".into())), ("speed", Tag::Float(2.0f32.to_bits())),
        ])]))]);
        assert_eq!(harvestable(&tool, "minecraft:stone", |_, _| false), Some(true));
        assert_eq!(evaluated_speed(&tool, "minecraft:stone", |_, _| false), Some(None));
    }

    #[test]
    fn actual_tool_matching_speed_is_returned() {
        let tool = Tag::compound([("rules", Tag::List(vec![Tag::compound([
            ("blocks", Tag::String("minecraft:stone".into())), ("speed", Tag::Float(2.0f32.to_bits())), ("correct_for_drops", Tag::Byte(1)),
        ])]))]);
        assert_eq!(effective_speed(&tool, "minecraft:stone", |_, _| false), Some(2.0));
        assert_eq!(harvestable(&tool, "minecraft:stone", |_, _| false), Some(true));
    }
}
