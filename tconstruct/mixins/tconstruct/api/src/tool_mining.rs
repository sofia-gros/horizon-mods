//! Tool mining behavior as a per-stack `minecraft:tool` component, from
//! MiningSpeedToolHook/IsEffectiveModule (SlimeKnights, MIT; see LICENSE).
//! The host's native mining rules read this component from the stack, so a rebuild
//! only has to write it. Only vanilla harvest tiers exist (no other-mod tiers).
use pluto_lib::{Identifier, Nbt, NbtCompound};

/// IsEffectiveModule with a `mantle:tag` predicate, the only form the original ships.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Effective { pub tag: Identifier, pub ignore_tier: bool }

impl Effective {
    pub fn decode(module: &serde_json::Map<String, serde_json::Value>) -> Result<Self, String> {
        // IsEffectiveModule's LOADER reads the predicate inline under predicate_type.
        if module.get("predicate_type").and_then(serde_json::Value::as_str) != Some("mantle:tag") {
            return Err("only mantle:tag effectiveness predicates are implemented".into());
        }
        let tag = Identifier::parse(module.get("tag").and_then(serde_json::Value::as_str).ok_or("tag must be an ID string")?)
            .map_err(|error| error.to_string())?;
        let ignore_tier = match module.get("ignore_tier") {
            None | Some(serde_json::Value::Null) => false,
            Some(serde_json::Value::Bool(value)) => *value,
            _ => return Err("ignore_tier must be boolean".into()),
        };
        Ok(Self { tag, ignore_tier })
    }
}

/// Vanilla's per-tier block tag naming the blocks that tier cannot harvest correctly.
fn incorrect_tag(tier: &Identifier) -> Option<&'static str> {
    if tier.namespace() != "minecraft" { return None; }
    Some(match tier.path() {
        "wood" => "#minecraft:incorrect_for_wooden_tool", "gold" => "#minecraft:incorrect_for_gold_tool",
        "stone" => "#minecraft:incorrect_for_stone_tool", "iron" => "#minecraft:incorrect_for_iron_tool",
        "diamond" => "#minecraft:incorrect_for_diamond_tool", "netherite" => "#minecraft:incorrect_for_netherite_tool",
        _ => return None,
    })
}

/// IsEffectiveToolHook for one block: it is in the effective tag and, unless tiers are ignored, the
/// tool's tier can harvest it. `in_tag` answers block tags.
pub(crate) fn is_effective(effective: &Effective, tier: &Identifier, in_tag: &dyn Fn(&Identifier) -> bool) -> bool {
    if !in_tag(&effective.tag) { return false; }
    if effective.ignore_tier { return true; }
    match incorrect_tag(tier).and_then(|tag| Identifier::parse(tag.trim_start_matches('#')).ok()) {
        Some(incorrect) => !in_tag(&incorrect),
        None => false,
    }
}

/// MiningSpeedToolHook.getDestroySpeed + isToolEffective as vanilla tool rules:
/// - broken: every block mines at 0.3 and nothing counts as correct for drops;
/// - otherwise an effective, correct-tier block mines at max(1, mining speed) and drops,
///   while everything else mines at 1 (a block the tier cannot harvest is matched first so it
///   neither speeds up nor drops, unlike a bare rule that would still apply its speed).
pub(crate) fn tool_component(effective: &Effective, broken: bool, mining_speed: f32, tier: &Identifier) -> Result<NbtCompound, String> {
    let mut rules = Vec::new();
    let default_speed = if broken { 0.3 } else { 1.0 };
    if !broken {
        if !effective.ignore_tier {
            let incorrect = incorrect_tag(tier).ok_or_else(|| format!("unknown harvest tier {tier}"))?;
            rules.push(rule(incorrect, Some(1.0), false));
        }
        rules.push(rule(&format!("#{}", effective.tag), Some(mining_speed.max(1.0)), true));
    }
    let mut tool = NbtCompound::new();
    tool.insert("rules".into(), Nbt::List(rules));
    tool.insert("default_mining_speed".into(), Nbt::Float(default_speed));
    tool.insert("damage_per_block".into(), Nbt::Int(1));
    Ok(tool)
}

fn rule(blocks: &str, speed: Option<f32>, correct_for_drops: bool) -> Nbt {
    let mut rule = NbtCompound::new();
    rule.insert("blocks".into(), Nbt::String(blocks.into()));
    if let Some(speed) = speed { rule.insert("speed".into(), Nbt::Float(speed)); }
    rule.insert("correct_for_drops".into(), Nbt::Byte(i8::from(correct_for_drops)));
    Nbt::Compound(rule)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn pickaxe() -> Effective { Effective::decode(json!({"predicate_type":"mantle:tag","tag":"minecraft:mineable/pickaxe"}).as_object().unwrap()).unwrap() }
    fn rules(tool: &NbtCompound) -> Vec<(String, Option<f32>, bool)> {
        let Nbt::List(rules) = &tool["rules"] else { panic!() };
        rules.iter().map(|rule| {
            let Nbt::Compound(rule) = rule else { panic!() };
            let Nbt::String(blocks) = &rule["blocks"] else { panic!() };
            (blocks.clone(), match rule.get("speed") { Some(Nbt::Float(v)) => Some(*v), _ => None },
             matches!(rule["correct_for_drops"], Nbt::Byte(1)))
        }).collect()
    }
    #[test]
    fn effectiveness_needs_the_tag_and_a_tier_that_can_harvest() {
        let iron = Identifier::parse("minecraft:iron").unwrap();
        let wood = Identifier::parse("minecraft:wood").unwrap();
        let in_tags = |tags: &'static [&'static str]| move |tag: &Identifier| tags.contains(&tag.to_string().as_str());
        let effective = pickaxe();
        let tag = effective.tag.to_string();
        let tag: &'static str = Box::leak(tag.into_boxed_str());
        assert!(is_effective(&effective, &iron, &in_tags(Box::leak(vec![tag].into_boxed_slice()))));
        assert!(!is_effective(&effective, &iron, &in_tags(&[])), "not in the effective tag");
        assert!(!is_effective(&effective, &wood, &in_tags(Box::leak(vec![tag, "minecraft:incorrect_for_wooden_tool"].into_boxed_slice()))), "the tier cannot harvest it");
        let loose = Effective { ignore_tier: true, ..effective.clone() };
        assert!(is_effective(&loose, &wood, &in_tags(Box::leak(vec![tag, "minecraft:incorrect_for_wooden_tool"].into_boxed_slice()))));
    }

    #[test]
    fn tier_rule_precedes_the_effective_rule_so_wrong_tier_blocks_stay_slow() {
        let iron = Identifier::parse("minecraft:iron").unwrap();
        let tool = tool_component(&pickaxe(), false, 6.5, &iron).unwrap();
        assert_eq!(rules(&tool), vec![
            ("#minecraft:incorrect_for_iron_tool".into(), Some(1.0), false),
            ("#minecraft:mineable/pickaxe".into(), Some(6.5), true)]);
        assert_eq!(tool["default_mining_speed"], Nbt::Float(1.0));
        // Speed is floored at 1, matching Math.max(1, ...).
        assert_eq!(rules(&tool_component(&pickaxe(), false, 0.2, &iron).unwrap())[1].1, Some(1.0));
    }
    #[test]
    fn broken_tools_mine_slowly_and_ignore_tier_drops_the_tier_rule() {
        let wood = Identifier::parse("minecraft:wood").unwrap();
        let broken = tool_component(&pickaxe(), true, 9.0, &wood).unwrap();
        assert!(rules(&broken).is_empty());
        assert_eq!(broken["default_mining_speed"], Nbt::Float(0.3));
        let loose = Effective { ignore_tier: true, ..pickaxe() };
        assert_eq!(rules(&tool_component(&loose, false, 4.0, &wood).unwrap()).len(), 1);
        assert!(tool_component(&pickaxe(), false, 4.0, &Identifier::parse("other:mithril").unwrap()).is_err());
    }
    #[test]
    fn only_the_shipped_tag_predicate_decodes() {
        for bad in [json!({"predicate_type":"mantle:block","block":"minecraft:stone"}), json!({"tag":"minecraft:mineable/axe"}),
                    json!({"predicate_type":"mantle:tag"}), json!({"predicate_type":"mantle:tag","tag":"x:y","ignore_tier":"yes"})] {
            assert!(Effective::decode(bad.as_object().unwrap()).is_err(), "{bad}");
        }
        assert!(Effective::decode(json!({"predicate_type":"mantle:tag","tag":"minecraft:mineable/axe","ignore_tier":true}).as_object().unwrap()).unwrap().ignore_tier);
    }
}
