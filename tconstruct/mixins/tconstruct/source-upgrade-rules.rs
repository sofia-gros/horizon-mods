// Incremental recipes for supported source stat upgrades.
use horizon_items::items::ItemStack;
use horizon_world::nbt::Tag;
use serde_json::Value;
use std::collections::{BTreeMap, HashMap, HashSet};

#[derive(Debug)]
pub(super) struct UpgradePlan {
    pub id: String,
    pub amount: i32,
    pub consumed: Vec<(usize, u32)>,
    pub leftover: Option<ItemStack>,
}

pub(super) struct ExtractionPlan {
    pub upgrade: UpgradePlan,
    pub sponge: ItemStack,
}

const UPGRADES: [(&str, i32); 8] = [
    ("tconstruct:haste", 45), ("tconstruct:sharpness", 36), ("tconstruct:swiftstrike", 72),
    ("tconstruct:quick_charge", 5), ("tconstruct:trueshot", 10),
    ("tconstruct:diamond", 1), ("tconstruct:emerald", 1), ("tconstruct:reinforced", 4),
];

pub(super) fn selection_id(index: usize) -> Option<&'static str> {
    UPGRADES.get(index).map(|(id, _)| *id)
}

pub(super) fn selection_index(id: &str) -> Option<usize> {
    UPGRADES.iter().position(|(candidate, _)| *candidate == id)
}

fn supported(id: &str, item: &str) -> bool {
    match id {
        "tconstruct:quick_charge" => item == "tconstruct:crossbow",
        "tconstruct:trueshot" => matches!(item, "tconstruct:longbow" | "tconstruct:crossbow"),
        _ => UPGRADES.iter().any(|(name, _)| *name == id),
    }
}

fn amount(tool: &ItemStack, id: &str) -> i32 {
    match tool
        .data()
        .and_then(|d| d.other("minecraft:custom_data"))
        .and_then(|n| n.get("tic_upgrades"))
        .and_then(|n| n.get(id))
    {
        Some(Tag::Int(value)) if *value > 0 => *value,
        _ => 0,
    }
}

pub(super) fn extractable(tool: &ItemStack) -> Vec<&'static str> {
    UPGRADES.into_iter().filter_map(|(id, needed)|
        (supported(id, &tool.item) && amount(tool, id) >= needed).then_some(id)).collect()
}

pub(super) fn effective(tool: &ItemStack, id: &str, needed: i32) -> f32 {
    amount(tool, id) as f32 / needed as f32
}

/// The upgrade extraction recipe returns the complete level and preserves any partial level.
pub(super) fn extract_upgrade(
    tags: &HashMap<String, HashSet<String>>, tool: &ItemStack,
    inputs: &[Option<ItemStack>], id: &str,
) -> Option<ExtractionPlan> {
    if !supported(id, &tool.item) { return None; }
    let needed = UPGRADES.iter().find_map(|(name, needed)| (*name == id).then_some(*needed))?;
    let belongs = |tag: &str| tags.get(tag).is_some_and(|items| items.contains(tool.item.as_ref()));
    if tool.count != 1 || !belongs("tconstruct:modifiable") || belongs("tconstruct:modifiable/unsalvageable") { return None; }
    let old = amount(tool, id);
    if old < needed { return None; }
    let present: Vec<_> = inputs.iter().enumerate().filter_map(|(i, input)| input.as_ref().map(|input| (i + 1, input))).collect();
    if present.len() != 2 { return None; }
    let slime = present.iter().find(|(_, item)| &*item.item == "tconstruct:sky_slime_crystal")?.0;
    let sponge = present.iter().find(|(_, item)| &*item.item == "minecraft:wet_sponge")?.0;
    let mut crystal = ItemStack::new("tconstruct:modifier_crystal", 1);
    crystal.data_mut().set_other("minecraft:custom_data", Some(Tag::compound([("modifier", Tag::String(id.into()))])));
    Some(ExtractionPlan { upgrade: UpgradePlan { id: id.into(), amount: old - needed,
        consumed: vec![(0, 1), (slime, 1), (sponge, 1)], leftover: Some(crystal) },
        sponge: ItemStack::new("minecraft:sponge", 1) })
}

pub(super) fn plan(
    recipes: &BTreeMap<String, Vec<u8>>,
    tags: &HashMap<String, HashSet<String>>,
    tool: &ItemStack,
    inputs: &[Option<ItemStack>],
    upgrade_slots: i32,
) -> Option<UpgradePlan> {
    let occupied = UPGRADES
        .iter()
        .map(|(id, needed)| (amount(tool, id) + needed - 1) / needed)
        .sum::<i32>();
    for bytes in recipes.values() {
        let Ok(value) = serde_json::from_slice::<Value>(bytes) else {
            continue;
        };
        let recipe_type = value.get("type").and_then(Value::as_str);
        if !matches!(recipe_type, Some("tconstruct:incremental_modifier" | "tconstruct:modifier")) { continue; }
        let Some(id) = value.get("result").and_then(Value::as_str) else {
            continue;
        };
        if !supported(id, &tool.item) {
            continue;
        }
        let Some(tools) = value.get("tools") else { continue; };
        let matches_tool = |entry: &Value| entry.get("tag").and_then(Value::as_str)
            .and_then(|tag| tags.get(tag)).is_some_and(|items| items.contains(tool.item.as_ref()));
        if !(matches_tool(tools) || tools.as_array().is_some_and(|entries| entries.iter().any(matches_tool))) {
            continue;
        }
        if recipe_type == Some("tconstruct:modifier") {
            if !matches!(id, "tconstruct:diamond" | "tconstruct:emerald") || amount(tool, id) != 0 { continue; }
            let cost = value.pointer("/slots/upgrades").and_then(Value::as_i64).and_then(|v| i32::try_from(v).ok()).unwrap_or(0);
            if occupied + cost > upgrade_slots || value.get("level").and_then(Value::as_i64) != Some(1) { continue; }
            let Some(ingredient) = value.get("inputs").and_then(Value::as_array).filter(|v| v.len() == 1).and_then(|v| v.first()) else { continue; };
            let present: Vec<_> = inputs.iter().enumerate().filter_map(|(i, stack)| stack.as_ref().map(|stack| (i + 1, stack))).collect();
            if present.len() != 1 { continue; }
            let (index, stack) = present[0];
            let crystal = value.get("allow_crystal").and_then(Value::as_bool).unwrap_or(true)
                && &*stack.item == "tconstruct:modifier_crystal"
                && matches!(stack.data().and_then(|d| d.other("minecraft:custom_data")).and_then(|d| d.get("modifier")), Some(Tag::String(value)) if value.to_string_lossy() == id);
            let ingredient_match = ingredient.get("item").and_then(Value::as_str).is_some_and(|item| item == &*stack.item)
                || ingredient.get("tag").and_then(Value::as_str).and_then(|tag| tags.get(tag)).is_some_and(|items| items.contains(stack.item.as_ref()));
            if !crystal && !ingredient_match { continue; }
            return Some(UpgradePlan { id: id.to_owned(), amount: 1, consumed: vec![(0, 1), (index, 1)], leftover: None });
        }
        let Some(input) = value.get("input") else {
            continue;
        };
        let matches = |stack: &ItemStack| -> bool {
            input
                .get("item")
                .and_then(Value::as_str)
                .is_some_and(|id| id == &*stack.item)
                || input.get("tag").and_then(Value::as_str).is_some_and(|tag| {
                    tags.get(tag)
                        .is_some_and(|items| items.contains(stack.item.as_ref()))
                })
        };
        let present: Vec<_> = inputs
            .iter()
            .enumerate()
            .filter_map(|(i, stack)| stack.as_ref().map(|stack| (i + 1, stack)))
            .collect();
        let crystal = value.get("allow_crystal").and_then(Value::as_bool).unwrap_or(true)
            && present.len() == 1 && &*present[0].1.item == "tconstruct:modifier_crystal"
            && matches!(present[0].1.data().and_then(|data| data.other("minecraft:custom_data"))
                .and_then(|data| data.get("modifier")), Some(Tag::String(value)) if value.to_string_lossy() == id);
        if present.is_empty() || (!crystal && present.iter().any(|(_, stack)| !matches(stack))) {
            continue;
        }
        let Some(needed) = value
            .get("needed_per_level")
            .and_then(Value::as_i64)
            .and_then(|v| i32::try_from(v).ok())
            .filter(|v| *v > 0)
        else {
            continue;
        };
        let Some(per_item) = value
            .get("amount_per_item")
            .and_then(Value::as_i64)
            .and_then(|v| i32::try_from(v).ok())
            .filter(|v| *v > 0)
        else {
            continue;
        };
        let Some(max) = value
            .pointer("/level/max")
            .and_then(Value::as_i64)
            .and_then(|v| i32::try_from(v).ok())
        else {
            continue;
        };
        let cost = value
            .pointer("/slots/upgrades")
            .and_then(Value::as_i64)
            .and_then(|v| i32::try_from(v).ok())
            .unwrap_or(0);
        let old = amount(tool, id);
        if crystal {
            if old.saturating_add(needed) > needed.saturating_mul(max) || occupied + cost > upgrade_slots { continue; }
            return Some(UpgradePlan { id: id.to_owned(), amount: old + needed,
                consumed: vec![(0, 1), (present[0].0, 1)], leftover: None });
        }
        if old >= needed * max || (old % needed == 0 && occupied + cost > upgrade_slots) {
            continue;
        }
        let remaining = needed - old % needed;
        let available = present
            .iter()
            .map(|(_, stack)| stack.count as i32 * per_item)
            .sum::<i32>();
        let used = available.min(remaining);
        if used <= 0 {
            continue;
        }
        let mut count = (used + per_item - 1) / per_item;
        let mut consumed = vec![(0, 1)];
        for (index, stack) in present {
            let take = (stack.count as i32).min(count);
            if take > 0 {
                consumed.push((index, take as u32));
                count -= take;
            }
        }
        let excess = (per_item - used % per_item) % per_item;
        let leftover = if excess > 0 {
            let name = value.get("leftover").and_then(Value::as_str)?;
            Some(ItemStack::new(name, excess as u32))
        } else {
            None
        };
        return Some(UpgradePlan {
            id: id.to_owned(),
            amount: old + used,
            consumed,
            leftover,
        });
    }
    None
}

/// Regenerate gem durability and armor from baseline, never from a previous upgrade result.
pub(super) fn apply_gem_durability_and_armor(tool: &mut ItemStack, stats: &super::tconstruct_source_station_stats::BaseStats,
    levels: &BTreeMap<String, i32>, armor: bool, worn_armor: bool) -> Option<()> {
    let diamond = amount(tool, "tconstruct:diamond") > 0;
    let emerald = amount(tool, "tconstruct:emerald") > 0;
    if !diamond && !emerald {
        let mut custom = tool.data()?.other("minecraft:custom_data")?.clone();
        let Tag::Compound(ref mut fields) = custom else { return None; };
        fields.remove(&"tconstruct:diamond_armor".into());
        fields.remove(&"tic_attack_conditional_multiplier".into());
        tool.data_mut().set_other("minecraft:custom_data", Some(custom));
        return Some(());
    }
    let trait_boost = 0.15 * *levels.get("tconstruct:preserved").unwrap_or(&0) as f32
        + 0.1 * *levels.get("tconstruct:ductile").unwrap_or(&0) as f32;
    let base = stats.durability + stats.durability_boost_unit * trait_boost
        + if diamond { (if armor { 250.0 } else { 500.0 }) * stats.durability_multiplier } else { 0.0 };
    let durability = (base * if emerald { 1.5 } else { 1.0 }) as i32;
    if durability < 1 { return None; }
    let mut custom = tool.data()?.other("minecraft:custom_data")?.clone();
    let Tag::Compound(ref mut fields) = custom else { return None; };
    fields.insert("tic_max_damage".into(), Tag::Int(durability));
    // Source multiplier scales eligible conditional bonuses, not the base attack value.
    if emerald { fields.insert("tic_attack_conditional_multiplier".into(), Tag::Float(1.25f32.to_bits())); }
    else { fields.remove(&"tic_attack_conditional_multiplier".into()); }
    if armor {
        if diamond { fields.insert("tic_armor".into(), Tag::Float((stats.armor + stats.armor_add_multiplier).clamp(0.0, 30.0).to_bits())); }
        if emerald {
            let ductile = *levels.get("tconstruct:ductile").unwrap_or(&0) as f32;
            fields.insert("tic_armor_toughness".into(), Tag::Float((stats.armor_toughness + ductile + stats.armor_toughness_add_multiplier).clamp(0.0, 20.0).to_bits()));
        }
    }
    if diamond && worn_armor { fields.insert("tconstruct:diamond_armor".into(), Tag::Byte(1)); }
    else { fields.remove(&"tconstruct:diamond_armor".into()); }
    tool.data_mut().set_other("minecraft:custom_data", Some(custom));
    tool.data_mut().set_other("minecraft:max_damage", Some(Tag::Int(durability)));
    Some(())
}

/// Rebuild additive and multiplier effects from the source baseline and saved progress.
pub(super) fn apply_stats(
    tool: &mut ItemStack,
    stats: &super::tconstruct_source_station_stats::BaseStats,
    levels: &std::collections::BTreeMap<String, i32>,
    melee: bool,
    melee_weapon: bool,
) -> Option<()> {
    let level = |id: &str| levels.get(id).copied().unwrap_or(0);
    let (damage, speed) = super::tconstruct_source_melee_stat_ops::melee_stats(
        super::tconstruct_source_melee_stat_ops::MeleeInputs {
            damage: stats.attack_damage + 0.5 * effective(tool, "tconstruct:diamond", 1).min(1.0) * stats.attack_damage_multiplier,
            damage_add_multiplier: stats.attack_damage_multiplier,
            speed: stats.attack_speed,
            ductile: level("tconstruct:ductile"), solid: level("tconstruct:solid"),
            pierce: level("tconstruct:pierce"), lightweight: level("tconstruct:lightweight"),
            vintage: level("tconstruct:vintage"),
            sharpness: effective(tool, "tconstruct:sharpness", 36),
            swiftstrike: effective(tool, "tconstruct:swiftstrike", 72),
            melee, melee_weapon,
        },
    );
    let mut custom = tool.data()?.other("minecraft:custom_data")?.clone();
    let Tag::Compound(ref mut fields) = custom else { return None; };
    if melee { fields.insert("tic_attack_damage".into(), Tag::Float(damage.to_bits())); }
    if melee_weapon { fields.insert("tic_attack_speed".into(), Tag::Float(speed.to_bits())); }
    let quick = effective(tool, "tconstruct:quick_charge", 5);
    if &*tool.item == "tconstruct:crossbow" && quick > 0.0 {
        let draw = (stats.draw_speed * (1.0 + 0.25 * quick)).clamp(0.0, 1024.0);
        fields.insert("tic_draw_speed".into(), Tag::Float(draw.to_bits()));
    }
    let true_level = effective(tool, "tconstruct:trueshot", 10);
    let emerald = effective(tool, "tconstruct:emerald", 1).min(1.0);
    if matches!(&*tool.item, "tconstruct:longbow" | "tconstruct:crossbow") && (true_level > 0.0 || emerald > 0.0) {
        let accuracy = (stats.accuracy + stats.accuracy_add_multiplier * 0.1 * (true_level + emerald)).clamp(0.1, 1.0);
        fields.insert("tic_accuracy".into(), Tag::Float(accuracy.to_bits()));
    }
    tool.data_mut().set_other("minecraft:custom_data", Some(custom));
    Some(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reinforced_partial_slot_crystal_and_limit_use_source_rules() {
        let bytes = br#"{"type":"tconstruct:incremental_modifier","allow_crystal":true,"amount_per_item":1,"input":{"item":"tconstruct:emerald_reinforcement"},"level":{"max":5},"needed_per_level":4,"result":"tconstruct:reinforced","slots":{"upgrades":1},"tools":{"tag":"tconstruct:modifiable/durability"}}"#.to_vec();
        let recipes = BTreeMap::from([("reinforced".into(),bytes)]);
        let tags = HashMap::from([
            ("tconstruct:modifiable/durability".into(),HashSet::from(["tconstruct:pickaxe".into()])),
            ("tconstruct:modifiable".into(),HashSet::from(["tconstruct:pickaxe".into()])),
        ]);
        let mut tool=ItemStack::new("tconstruct:pickaxe",1);
        let ingredients=[Some(ItemStack::new("tconstruct:emerald_reinforcement",3))];
        assert!(plan(&recipes,&tags,&tool,&ingredients,0).is_none());
        let partial=plan(&recipes,&tags,&tool,&ingredients,1).unwrap();assert_eq!(partial.amount,3);
        tool.data_mut().set_other("minecraft:custom_data",Some(Tag::compound([("tic_upgrades",Tag::compound([("tconstruct:reinforced",Tag::Int(3))]))])));
        assert_eq!(plan(&recipes,&tags,&tool,&[Some(ItemStack::new("tconstruct:emerald_reinforcement",4))],1).unwrap().amount,4);
        let mut crystal=ItemStack::new("tconstruct:modifier_crystal",1);crystal.data_mut().set_other("minecraft:custom_data",Some(Tag::compound([("modifier",Tag::String("tconstruct:reinforced".into()))])));
        assert!(plan(&recipes,&tags,&tool,&[Some(crystal.clone())],1).is_none());
        assert_eq!(plan(&recipes,&tags,&tool,&[Some(crystal)],2).unwrap().amount,7);
        tool.data_mut().set_other("minecraft:custom_data",Some(Tag::compound([("tic_upgrades",Tag::compound([("tconstruct:reinforced",Tag::Int(7))]))])));
        assert_eq!(extract_upgrade(&tags,&tool,&[Some(ItemStack::new("minecraft:wet_sponge",1)),Some(ItemStack::new("tconstruct:sky_slime_crystal",1))],"tconstruct:reinforced").unwrap().upgrade.amount,3);
        tool.data_mut().set_other("minecraft:custom_data",Some(Tag::compound([("tic_upgrades",Tag::compound([("tconstruct:reinforced",Tag::Int(20))]))])));
        assert!(plan(&recipes,&tags,&tool,&ingredients,10).is_none());
    }

    #[test]
    fn gems_recalculate_durability_armor_and_accuracy_without_stacking() {
        let def = br#"{"modules":[{"type":"tconstruct:base_stats","stats":{"tconstruct:durability":100,"tconstruct:armor":2,"tconstruct:armor_toughness":1,"tconstruct:accuracy":0.75}}]}"#;
        let stats = super::super::tconstruct_source_station_stats::base_stats(def, &[]).unwrap();
        let mut tool = ItemStack::new("tconstruct:crossbow", 1);
        tool.data_mut().set_other("minecraft:custom_data", Some(Tag::compound([("tic_upgrades", Tag::compound([("tconstruct:diamond",Tag::Int(1)),("tconstruct:emerald",Tag::Int(1))]))])));
        apply_gem_durability_and_armor(&mut tool,&stats,&BTreeMap::new(),false,false).unwrap();
        apply_stats(&mut tool,&stats,&BTreeMap::new(),true,false).unwrap();
        let custom = tool.data().unwrap().other("minecraft:custom_data").unwrap();
        assert!(matches!(custom.get("tic_max_damage"), Some(Tag::Int(900))));
        assert!(matches!(custom.get("tic_attack_conditional_multiplier"), Some(Tag::Float(v)) if (f32::from_bits(*v)-1.25).abs()<0.0001));
        assert!(matches!(custom.get("tic_accuracy"), Some(Tag::Float(v)) if (f32::from_bits(*v)-0.85).abs()<0.0001));
        apply_gem_durability_and_armor(&mut tool,&stats,&BTreeMap::new(),true,true).unwrap();
        let custom = tool.data().unwrap().other("minecraft:custom_data").unwrap();
        assert!(matches!(custom.get("tic_max_damage"), Some(Tag::Int(525))));
        assert!(matches!(custom.get("tic_armor"), Some(Tag::Float(v)) if (f32::from_bits(*v)-3.0).abs()<0.0001));
        assert!(matches!(custom.get("tic_armor_toughness"), Some(Tag::Float(v)) if (f32::from_bits(*v)-2.0).abs()<0.0001));
        assert!(matches!(custom.get("tconstruct:diamond_armor"), Some(Tag::Byte(1))));
        assert_eq!(super::super::tconstruct_source_station_stats::gem_tier(Some("netherite"),true,true),Some("netherite"));
        assert_eq!(super::super::tconstruct_source_station_stats::gem_tier(Some("stone"),false,true),Some("iron"));
        assert_eq!(super::super::tconstruct_source_station_stats::gem_tier(Some("stone"),true,true),Some("diamond"));
    }

    #[test]
    fn single_gems_require_tags_one_slot_and_cannot_repeat() {
        let recipe = br#"{"type":"tconstruct:modifier","allow_crystal":true,"inputs":[{"tag":"forge:gems/diamond"}],"level":1,"result":"tconstruct:diamond","slots":{"upgrades":1},"tools":{"tag":"tconstruct:modifiable/durability"}}"#.to_vec();
        let recipes = BTreeMap::from([("diamond".into(), recipe)]);
        let tags = HashMap::from([
            ("forge:gems/diamond".into(), HashSet::from(["minecraft:diamond".into()])),
            ("tconstruct:modifiable/durability".into(), HashSet::from(["tconstruct:pickaxe".into()])),
            ("tconstruct:modifiable".into(), HashSet::from(["tconstruct:pickaxe".into()])),
        ]);
        let mut tool = ItemStack::new("tconstruct:pickaxe", 1);
        let inputs = [Some(ItemStack::new("minecraft:diamond", 3))];
        assert_eq!(plan(&recipes, &tags, &tool, &inputs, 1).unwrap().consumed, [(0,1),(1,1)]);
        assert!(plan(&recipes, &tags, &tool, &inputs, 0).is_none());
        assert!(plan(&recipes, &tags, &ItemStack::new("minecraft:bow", 1), &inputs, 1).is_none());
        tool.data_mut().set_other("minecraft:custom_data", Some(Tag::compound([("tic_upgrades", Tag::compound([("tconstruct:diamond",Tag::Int(1))]))])));
        assert!(plan(&recipes, &tags, &tool, &inputs, 2).is_none());
        let extraction = extract_upgrade(&tags, &tool, &[Some(ItemStack::new("minecraft:wet_sponge",1)), Some(ItemStack::new("tconstruct:sky_slime_crystal",1))], "tconstruct:diamond").unwrap();
        assert_eq!(extraction.upgrade.amount, 0);
        let mut clear = ItemStack::new("tconstruct:pickaxe",1);
        assert_eq!(plan(&recipes, &tags, &clear, &[extraction.upgrade.leftover], 1).unwrap().amount,1);
        clear.count = 1;
    }

    #[test]
    fn ranged_transactions_share_slots_crystals_and_source_limits() {
        let quick = br#"{"type":"tconstruct:incremental_modifier","allow_crystal":true,"amount_per_item":1,"input":{"item":"minecraft:magma_cream"},"level":{"max":4},"needed_per_level":5,"result":"tconstruct:quick_charge","slots":{"upgrades":1},"tools":{"tag":"tconstruct:modifiable/ranged/quick_charge"}}"#.to_vec();
        let true_recipe = br#"{"type":"tconstruct:incremental_modifier","allow_crystal":true,"amount_per_item":1,"input":{"item":"minecraft:target"},"level":{"max":3},"needed_per_level":10,"result":"tconstruct:trueshot","slots":{"upgrades":1},"tools":{"tag":"tconstruct:modifiable/ranged"}}"#.to_vec();
        let recipes = BTreeMap::from([("quick".into(), quick), ("true".into(), true_recipe)]);
        let tags = HashMap::from([
            ("tconstruct:modifiable/ranged/quick_charge".into(), HashSet::from(["tconstruct:crossbow".into(), "tconstruct:war_pick".into()])),
            ("tconstruct:modifiable/ranged".into(), HashSet::from(["tconstruct:crossbow".into(), "tconstruct:longbow".into(), "tconstruct:javelin".into()])),
            ("tconstruct:modifiable".into(), HashSet::from(["tconstruct:crossbow".into(), "tconstruct:longbow".into()])),
        ]);
        let mut bow = ItemStack::new("tconstruct:crossbow", 1);
        bow.data_mut().set_other("minecraft:custom_data", Some(Tag::compound([("tic_upgrades", Tag::compound([
            ("tconstruct:sharpness", Tag::Int(36)), ("tconstruct:quick_charge", Tag::Int(2)),
        ]))])));
        let result = plan(&recipes, &tags, &bow, &[Some(ItemStack::new("minecraft:magma_cream", 9))], 2).unwrap();
        assert_eq!(result.amount, 5);
        assert_eq!(result.consumed, [(0, 1), (1, 3)]);
        assert!(plan(&recipes, &tags, &bow, &[Some(ItemStack::new("minecraft:target", 1))], 2).is_none());
        let mut crystal = ItemStack::new("tconstruct:modifier_crystal", 1);
        crystal.data_mut().set_other("minecraft:custom_data", Some(Tag::compound([("modifier", Tag::String("tconstruct:quick_charge".into()))])));
        assert!(plan(&recipes, &tags, &bow, &[Some(crystal.clone())], 2).is_none());
        assert_eq!(plan(&recipes, &tags, &bow, &[Some(crystal)], 3).unwrap().amount, 7);
        let mut full = bow.clone();
        full.data_mut().set_other("minecraft:custom_data", Some(Tag::compound([("tic_upgrades", Tag::compound([("tconstruct:quick_charge", Tag::Int(20))]))])));
        assert!(plan(&recipes, &tags, &full, &[Some(ItemStack::new("minecraft:magma_cream", 1))], 9).is_none());
        let mut longbow = ItemStack::new("tconstruct:longbow", 1);
        longbow.data_mut().set_other("minecraft:custom_data", Some(Tag::compound([("tic_upgrades", Tag::compound([("tconstruct:trueshot", Tag::Int(30))]))])));
        assert!(plan(&recipes, &tags, &longbow, &[Some(ItemStack::new("minecraft:target", 1))], 9).is_none());
        let partial = plan(&recipes, &tags, &ItemStack::new("tconstruct:longbow", 1), &[Some(ItemStack::new("minecraft:target", 3))], 1).unwrap();
        assert_eq!(partial.amount, 3);
        assert_eq!(partial.consumed, [(0, 1), (1, 3)]);
        longbow.data_mut().set_other("minecraft:custom_data", Some(Tag::compound([("tic_upgrades", Tag::compound([("tconstruct:trueshot", Tag::Int(13))]))])));
        let taken = extract_upgrade(&tags, &longbow, &[Some(ItemStack::new("minecraft:wet_sponge", 1)), Some(ItemStack::new("tconstruct:sky_slime_crystal", 1))], "tconstruct:trueshot").unwrap();
        assert_eq!(taken.upgrade.amount, 3);
        longbow.data_mut().set_other("minecraft:custom_data", Some(Tag::compound([("tic_upgrades", Tag::compound([("tconstruct:trueshot", Tag::Int(3))]))])));
        assert_eq!(plan(&recipes, &tags, &longbow, &[taken.upgrade.leftover], 2).unwrap().amount, 13);
        let extracted = extract_upgrade(&tags, &full, &[Some(ItemStack::new("minecraft:wet_sponge", 1)), Some(ItemStack::new("tconstruct:sky_slime_crystal", 1))], "tconstruct:quick_charge").unwrap();
        assert_eq!(extracted.upgrade.amount, 15);
        let mut original = bow.clone();
        original.data_mut().set_other("minecraft:custom_data", Some(Tag::compound([("tic_upgrades", Tag::compound([("tconstruct:quick_charge", Tag::Int(7))]))])));
        let extracted = extract_upgrade(&tags, &original, &[Some(ItemStack::new("minecraft:wet_sponge", 1)), Some(ItemStack::new("tconstruct:sky_slime_crystal", 1))], "tconstruct:quick_charge").unwrap();
        assert_eq!(extracted.upgrade.amount, 2);
        let mut reduced = original.clone();
        reduced.data_mut().set_other("minecraft:custom_data", Some(Tag::compound([("tic_upgrades", Tag::compound([("tconstruct:quick_charge", Tag::Int(extracted.upgrade.amount))]))])));
        let reapplied = plan(&recipes, &tags, &reduced, &[extracted.upgrade.leftover], 2).unwrap();
        assert_eq!(reapplied.amount, 7);
        assert_eq!(selection_index("tconstruct:trueshot").and_then(selection_id), Some("tconstruct:trueshot"));
        assert!(plan(&recipes, &tags, &ItemStack::new("tconstruct:war_pick", 1), &[Some(ItemStack::new("minecraft:magma_cream", 1))], 3).is_none());
        assert!(plan(&recipes, &tags, &ItemStack::new("tconstruct:javelin", 1), &[Some(ItemStack::new("minecraft:target", 1))], 3).is_none());
    }

    #[test]
    fn fractional_ranged_stats_use_source_order_and_accuracy_cap() {
        let definition = br#"{"modules":[{"type":"tconstruct:multiply_stats","multipliers":{"tconstruct:draw_speed":1.5,"tconstruct:accuracy":1.2}}]}"#;
        let stats = super::super::tconstruct_source_station_stats::base_stats(definition, &[]).unwrap();
        let mut crossbow = ItemStack::new("tconstruct:crossbow", 1);
        crossbow.data_mut().set_other("minecraft:custom_data", Some(Tag::compound([("tic_upgrades", Tag::compound([
            ("tconstruct:quick_charge", Tag::Int(2)), ("tconstruct:trueshot", Tag::Int(3)),
        ]))])));
        for _ in 0..2 {
            apply_stats(&mut crossbow, &stats, &BTreeMap::new(), false, false).unwrap();
            let data = crossbow.data().unwrap().other("minecraft:custom_data").unwrap();
            let (Some(Tag::Float(draw)), Some(Tag::Float(accuracy))) = (data.get("tic_draw_speed"), data.get("tic_accuracy")) else { panic!() };
            assert!((f32::from_bits(*draw) - 1.65).abs() < 0.00001);
            assert!((f32::from_bits(*accuracy) - 0.936).abs() < 0.00001);
        }
        crossbow.data_mut().set_other("minecraft:custom_data", Some(Tag::compound([("tic_upgrades", Tag::compound([("tconstruct:trueshot", Tag::Int(30))]))])));
        apply_stats(&mut crossbow, &stats, &BTreeMap::new(), false, false).unwrap();
        let Some(Tag::Float(accuracy)) = crossbow.data().unwrap().other("minecraft:custom_data").unwrap().get("tic_accuracy") else { panic!() };
        assert_eq!(f32::from_bits(*accuracy), 1.0);
    }

    fn tool(amount: i32) -> ItemStack {
        let mut tool = ItemStack::new("tconstruct:pickaxe", 1);
        tool.data_mut().set_other(
            "minecraft:custom_data",
            Some(Tag::compound([(
                "tic_upgrades",
                Tag::compound([("tconstruct:sharpness", Tag::Int(amount))]),
            )])),
        );
        tool
    }

    #[test]
    fn partial_levels_apply_source_damage_and_attack_speed() {
        let definition = br#"{"modules":[{"type":"tconstruct:part_stats","parts":["tconstruct:pick_head","tconstruct:tool_handle"]},{"type":"tconstruct:base_stats","stats":{"tconstruct:attack_speed":1.2}}]}"#;
        let head = serde_json::json!({"tconstruct:head":{"durability":100,"melee_attack":4}})
            .as_object()
            .unwrap()
            .clone();
        let handle = serde_json::json!({"tconstruct:handle":{"melee_damage":0.25}})
            .as_object()
            .unwrap()
            .clone();
        let stats =
            super::super::tconstruct_source_station_stats::base_stats(definition, &[head, handle])
                .unwrap();
        let mut tool = ItemStack::new("tconstruct:pickaxe", 1);
        tool.data_mut().set_other(
            "minecraft:custom_data",
            Some(Tag::compound([
                ("tic_attack_damage", Tag::Float(5.0f32.to_bits())),
                ("tic_attack_speed", Tag::Float(1.2f32.to_bits())),
                (
                    "tic_upgrades",
                    Tag::compound([
                        ("tconstruct:sharpness", Tag::Int(18)),
                        ("tconstruct:swiftstrike", Tag::Int(36)),
                    ]),
                ),
            ])),
        );
        apply_stats(&mut tool, &stats, &std::collections::BTreeMap::new(), true, true).unwrap();
        let data = tool.data().unwrap().other("minecraft:custom_data").unwrap();
        let Some(Tag::Float(damage)) = data.get("tic_attack_damage") else {
            panic!("missing damage")
        };
        let Some(Tag::Float(speed)) = data.get("tic_attack_speed") else {
            panic!("missing speed")
        };
        assert!((f32::from_bits(*damage) - 5.46875).abs() < 0.00001);
        assert!((f32::from_bits(*speed) - 1.23).abs() < 0.00001);
    }

    #[test]
    fn trait_levels_rebuild_without_double_application_or_crossing_tag_boundaries() {
        let definition = br#"{"modules":[{"type":"tconstruct:part_stats","parts":["tconstruct:pick_head","tconstruct:tool_handle"]},{"type":"tconstruct:base_stats","stats":{"tconstruct:attack_speed":1.2}}]}"#;
        let head = serde_json::json!({"tconstruct:head":{"durability":100,"melee_attack":4}}).as_object().unwrap().clone();
        let handle = serde_json::json!({"tconstruct:handle":{"melee_damage":0.25}}).as_object().unwrap().clone();
        let stats = super::super::tconstruct_source_station_stats::base_stats(definition, &[head, handle]).unwrap();
        let mut tool = tool(18);
        let levels = BTreeMap::from([
            ("tconstruct:ductile".to_string(), 2), ("tconstruct:solid".to_string(), 1),
            ("tconstruct:pierce".to_string(), 1), ("tconstruct:lightweight".to_string(), 2),
            ("tconstruct:vintage".to_string(), 1),
        ]);
        apply_stats(&mut tool, &stats, &levels, true, true).unwrap();
        let first = tool.data().unwrap().other("minecraft:custom_data").unwrap().clone();
        apply_stats(&mut tool, &stats, &levels, true, true).unwrap();
        assert_eq!(&first, tool.data().unwrap().other("minecraft:custom_data").unwrap());
        let Tag::Float(damage) = first.get("tic_attack_damage").unwrap() else { panic!() };
        let Tag::Float(speed) = first.get("tic_attack_speed").unwrap() else { panic!() };
        assert!((f32::from_bits(*damage) - 7.708594).abs() < 0.0001);
        assert!((f32::from_bits(*speed) - 1.2528).abs() < 0.0001);
        // Ineligible tools retain their preexisting values.
        apply_stats(&mut tool, &stats, &BTreeMap::new(), false, false).unwrap();
        assert_eq!(&first, tool.data().unwrap().other("minecraft:custom_data").unwrap());
    }

    #[test]
    fn swiftstrike_uses_both_input_slots_and_source_tool_tag() {
        let recipe = br#"{"type":"tconstruct:incremental_modifier","amount_per_item":1,"needed_per_level":72,"input":{"item":"minecraft:amethyst_shard"},"result":"tconstruct:swiftstrike","level":{"max":5},"slots":{"upgrades":1},"tools":{"tag":"tconstruct:modifiable/melee/weapon"}}"#.to_vec();
        let recipes = BTreeMap::from([("swift".into(), recipe)]);
        let tags = HashMap::from([(
            "tconstruct:modifiable/melee/weapon".into(),
            HashSet::from(["tconstruct:pickaxe".into()]),
        )]);
        let inputs = [
            Some(ItemStack::new("minecraft:amethyst_shard", 64)),
            Some(ItemStack::new("minecraft:amethyst_shard", 8)),
        ];
        let output = plan(&recipes, &tags, &tool(0), &inputs, 3).unwrap();
        assert_eq!(output.amount, 72);
        assert_eq!(output.consumed, [(0, 1), (1, 64), (2, 8)]);
        assert!(plan(&recipes, &HashMap::new(), &tool(0), &inputs, 3).is_none());
        let third = [None, None, Some(ItemStack::new("minecraft:amethyst_shard", 8))];
        let output = plan(&recipes, &tags, &tool(0), &third, 3).unwrap();
        assert_eq!(output.amount, 8);
        assert_eq!(output.consumed, [(0, 1), (3, 8)]);
    }

    #[test]
    fn incremental_slots_and_block_remainder_follow_source_recipe() {
        let shard = br#"{"type":"tconstruct:incremental_modifier","amount_per_item":1,"needed_per_level":36,"input":{"tag":"forge:gems/quartz"},"result":"tconstruct:sharpness","level":{"max":5},"slots":{"upgrades":1},"tools":{"tag":"tconstruct:modifiable/melee"}}"#.to_vec();
        let block = br#"{"type":"tconstruct:incremental_modifier","amount_per_item":4,"needed_per_level":36,"input":{"item":"minecraft:quartz_block"},"leftover":"minecraft:quartz","result":"tconstruct:sharpness","level":{"max":5},"slots":{"upgrades":1},"tools":{"tag":"tconstruct:modifiable/melee"}}"#.to_vec();
        let tags = HashMap::from([
            (
                "forge:gems/quartz".into(),
                HashSet::from(["minecraft:quartz".into()]),
            ),
            (
                "tconstruct:modifiable/melee".into(),
                HashSet::from(["tconstruct:pickaxe".into()]),
            ),
        ]);
        let recipes = BTreeMap::from([("shard".into(), shard), ("block".into(), block)]);
        let inputs = [Some(ItemStack::new("minecraft:quartz", 35)), None];
        let first = plan(&recipes, &tags, &tool(0), &inputs, 1).unwrap();
        assert_eq!(first.amount, 35);
        assert_eq!(first.consumed, [(0, 1), (1, 35)]);
        let last = plan(
            &recipes,
            &tags,
            &tool(35),
            &[Some(ItemStack::new("minecraft:quartz_block", 1)), None],
            1,
        )
        .unwrap();
        assert_eq!(last.amount, 36);
        assert_eq!(last.leftover.unwrap().count, 3);
        assert!(
            plan(
                &recipes,
                &tags,
                &tool(36),
                &[Some(ItemStack::new("minecraft:quartz", 1)), None],
                1
            )
            .is_none()
        );
        assert!(
            plan(
                &recipes,
                &tags,
                &tool(0),
                &[Some(ItemStack::new("minecraft:dirt", 1)), None],
                1
            )
            .is_none()
        );
        assert!(plan(&recipes, &tags, &tool(0), &inputs, 0).is_none());
        assert!(
            plan(
                &recipes,
                &tags,
                &tool(180),
                &[Some(ItemStack::new("minecraft:quartz", 1)), None],
                6
            )
            .is_none()
        );
    }

    #[test]
    fn haste_uses_array_tags_partial_progress_and_shared_slots() {
        let recipe = br#"{"type":"tconstruct:incremental_modifier","amount_per_item":9,"needed_per_level":45,"input":{"item":"minecraft:redstone_block"},"leftover":"minecraft:redstone","result":"tconstruct:haste","level":{"max":5},"slots":{"upgrades":1},"tools":[{"tag":"tconstruct:modifiable/harvest"},{"tag":"tconstruct:modifiable/armor/chestplate"}]}"#.to_vec();
        let recipes = BTreeMap::from([("haste".into(), recipe)]);
        let tags = HashMap::from([("tconstruct:modifiable/harvest".into(), HashSet::from(["tconstruct:pickaxe".into()]))]);
        let mut item = tool(36);
        item.data_mut().set_other("minecraft:custom_data", Some(Tag::compound([("tic_upgrades", Tag::compound([
            ("tconstruct:sharpness", Tag::Int(36)), ("tconstruct:haste", Tag::Int(44)),
        ]))])));
        let inputs = [None, None, Some(ItemStack::new("minecraft:redstone_block", 1))];
        let result = plan(&recipes, &tags, &item, &inputs, 2).unwrap();
        assert_eq!(result.amount, 45);
        assert_eq!(result.consumed, [(0, 1), (3, 1)]);
        assert_eq!(result.leftover.unwrap().count, 8);
        item.data_mut().set_other("minecraft:custom_data", Some(Tag::compound([("tic_upgrades", Tag::compound([
            ("tconstruct:sharpness", Tag::Int(36)), ("tconstruct:haste", Tag::Int(45)),
        ]))])));
        assert!(plan(&recipes, &tags, &item, &inputs, 2).is_none());
    }

    #[test]
    fn crystal_adds_one_level_preserving_partial_and_requires_a_slot() {
        let recipe = br#"{"type":"tconstruct:incremental_modifier","allow_crystal":true,"amount_per_item":1,"needed_per_level":36,"input":{"item":"minecraft:quartz"},"result":"tconstruct:sharpness","level":{"max":5},"slots":{"upgrades":1},"tools":{"tag":"tconstruct:modifiable/melee"}}"#.to_vec();
        let recipes = BTreeMap::from([("sharpness".into(), recipe)]);
        let tags = HashMap::from([("tconstruct:modifiable/melee".into(), HashSet::from(["tconstruct:pickaxe".into()]))]);
        let mut crystal = ItemStack::new("tconstruct:modifier_crystal", 2);
        crystal.data_mut().set_other("minecraft:custom_data", Some(Tag::compound([("modifier", Tag::String("tconstruct:sharpness".into()))])));
        let inputs = [Some(crystal.clone())];
        let result = plan(&recipes, &tags, &tool(18), &inputs, 2).unwrap();
        assert_eq!(result.amount, 54);
        assert_eq!(result.consumed, [(0, 1), (1, 1)]);
        assert!(plan(&recipes, &tags, &tool(18), &inputs, 1).is_none());
        assert!(plan(&recipes, &tags, &tool(162), &inputs, 9).is_none());
        assert!(plan(&recipes, &tags, &tool(18), &[Some(crystal.clone()), Some(crystal)], 9).is_none());
        let mut wrong = ItemStack::new("tconstruct:modifier_crystal", 1);
        wrong.data_mut().set_other("minecraft:custom_data", Some(Tag::compound([("modifier", Tag::String("tconstruct:haste".into()))])));
        assert!(plan(&recipes, &tags, &tool(0), &[Some(wrong)], 9).is_none());
        let block = br#"{"type":"tconstruct:incremental_modifier","allow_crystal":false,"amount_per_item":4,"needed_per_level":36,"input":{"item":"minecraft:quartz_block"},"result":"tconstruct:sharpness","level":{"max":5},"slots":{"upgrades":1},"tools":{"tag":"tconstruct:modifiable/melee"}}"#.to_vec();
        assert!(plan(&BTreeMap::from([("block".into(), block)]), &tags, &tool(0), &inputs, 9).is_none());
    }

    #[test]
    fn extraction_requires_a_complete_level_and_retains_partial_progress() {
        let tags = HashMap::from([("tconstruct:modifiable".into(), HashSet::from(["tconstruct:pickaxe".into()]))]);
        let inputs = [Some(ItemStack::new("minecraft:wet_sponge", 1)), Some(ItemStack::new("tconstruct:sky_slime_crystal", 1))];
        assert!(extract_upgrade(&tags, &tool(18), &inputs, "tconstruct:sharpness").is_none());
        let plan = extract_upgrade(&tags, &tool(54), &inputs, "tconstruct:sharpness").unwrap();
        assert_eq!(plan.upgrade.amount, 18);
        assert_eq!(plan.upgrade.consumed, [(0, 1), (2, 1), (1, 1)]);
        assert_eq!(&*plan.sponge.item, "minecraft:sponge");
        let crystal = plan.upgrade.leftover.unwrap();
        assert_eq!(&*crystal.item, "tconstruct:modifier_crystal");
        assert!(matches!(crystal.data().unwrap().other("minecraft:custom_data").unwrap().get("modifier"), Some(Tag::String(id)) if id.to_string_lossy() == "tconstruct:sharpness"));
    }
}
