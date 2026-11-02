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

const UPGRADES: [(&str, i32); 12] = [
    ("tconstruct:haste", 45), ("tconstruct:sharpness", 36), ("tconstruct:swiftstrike", 72),
    ("tconstruct:quick_charge", 5), ("tconstruct:trueshot", 10),
    ("tconstruct:diamond", 1), ("tconstruct:emerald", 1), ("tconstruct:reinforced", 4),
    ("tconstruct:netherite", 1), ("tconstruct:unbreakable", 1), ("tconstruct:overforced", 4),
    ("tconstruct:glowing", 1),
];

// Original slotless modifiers needed to make higher-level upgrades reachable.
const BONUS_SLOTS: [(&str, i32, i32); 6] = [
    ("tconstruct:writable", 1, 0), ("tconstruct:harmonious", 1, 0),
    ("tconstruct:recapitated", 1, 0), ("tconstruct:forecast", 1, 0),
    ("tconstruct:draconic", 0, 1),
    ("tconstruct:enhanced", 1, 0),
];

fn ingredient_matches(value: &Value, item: &str, tags: &HashMap<String, HashSet<String>>) -> bool {
    if let Some(entries) = value.as_array() { return entries.iter().any(|v| ingredient_matches(v, item, tags)); }
    match value.get("type").and_then(Value::as_str) {
        Some("forge:difference") => return value.get("base").is_some_and(|v| ingredient_matches(v, item, tags)) && !value.get("subtracted").is_some_and(|v| ingredient_matches(v, item, tags)),
        Some("forge:intersection") => return value.get("children").and_then(Value::as_array).is_some_and(|v| !v.is_empty() && v.iter().all(|v| ingredient_matches(v, item, tags))),
        Some(_) => return false,
        None => {}
    }
    value.get("item").and_then(Value::as_str).is_some_and(|v| v == item)
        || value.get("tag").and_then(Value::as_str).and_then(|tag| tags.get(tag)).is_some_and(|v| v.contains(item))
        || value.get("ingredient").is_some_and(|v| ingredient_matches(v, item, tags))
}

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
        _ => UPGRADES.iter().any(|(name, _)| *name == id) || BONUS_SLOTS.iter().any(|(name, _, _)| *name == id),
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

/// Recompute free upgrade and ability slots after material traits change.
fn is_ability(id: &str) -> bool { matches!(id, "tconstruct:unbreakable" | "tconstruct:glowing") }
fn used_abilities(tool: &ItemStack) -> i32 { UPGRADES.iter().filter(|(id, _)| is_ability(id)).map(|(id, needed)| (amount(tool,id) + needed - 1) / needed).sum() }

fn slot_state(tool: &ItemStack, traits: &BTreeMap<String, i32>, upgrades: i32, abilities: i32) -> (i32, i32, i32) {
    let bonus = |id: &str| amount(tool, id) + traits.get(id).copied().unwrap_or(0);
    let available_upgrades = upgrades + BONUS_SLOTS.iter().map(|(id, n, _)| bonus(id) * n).sum::<i32>();
    let available_abilities = abilities + BONUS_SLOTS.iter().map(|(id, _, n)| bonus(id) * n).sum::<i32>();
    let used_upgrades = UPGRADES.iter().filter(|(id, _)| !is_ability(id))
        .map(|(id, needed)| (amount(tool, id) + needed - 1) / needed).sum::<i32>();
    (available_upgrades, available_abilities, used_upgrades)
}
pub(super) fn valid_slots(tool: &ItemStack, traits: &BTreeMap<String, i32>, upgrades: i32, abilities: i32) -> bool {
    let (available_upgrades, available_abilities, used_upgrades) = slot_state(tool, traits, upgrades, abilities);
    used_upgrades <= available_upgrades && used_abilities(tool) <= available_abilities
}

/// Validate source Unbreakable dependencies against complete levels and material traits.
pub(super) fn valid_unbreakable(tool: &ItemStack, traits: &BTreeMap<String, i32>) -> bool {
    let level = |id: &str, needed: i32| amount(tool, id) / needed + traits.get(id).copied().unwrap_or(0);
    level("tconstruct:unbreakable", 1) == 0 ||
        (level("tconstruct:netherite", 1) >= 1 && level("tconstruct:reinforced", 4) >= 5)
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
    extract_upgrade_with_traits(tags, tool, inputs, id, &BTreeMap::new())
}

pub(super) fn extract_upgrade_with_traits(
    tags: &HashMap<String, HashSet<String>>, tool: &ItemStack,
    inputs: &[Option<ItemStack>], id: &str, traits: &BTreeMap<String, i32>,
) -> Option<ExtractionPlan> {
    if !supported(id, &tool.item) { return None; }
    let needed = UPGRADES.iter().find_map(|(name, needed)| (*name == id).then_some(*needed))?;
    let belongs = |tag: &str| tags.get(tag).is_some_and(|items| items.contains(tool.item.as_ref()));
    if tool.count != 1 || !belongs("tconstruct:modifiable") || belongs("tconstruct:modifiable/unsalvageable") { return None; }
    let old = amount(tool, id);
    if old < needed { return None; }
    // Validation applies to the rebuilt tool, including material traits.
    if amount(tool, "tconstruct:unbreakable") > 0 || traits.get("tconstruct:unbreakable").copied().unwrap_or(0) > 0 {
        let netherite = amount(tool, "tconstruct:netherite") - if id == "tconstruct:netherite" { needed } else { 0 } + traits.get("tconstruct:netherite").copied().unwrap_or(0);
        let reinforced_amount = amount(tool, "tconstruct:reinforced") - if id == "tconstruct:reinforced" { needed } else { 0 };
        let reinforced = reinforced_amount / 4 + traits.get("tconstruct:reinforced").copied().unwrap_or(0);
        if netherite < 1 || reinforced < 5 { return None; }
    }
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
    plan_with_traits(recipes, tags, tool, inputs, upgrade_slots, 0, &BTreeMap::new())
}

pub(super) fn plan_with_traits(
    recipes: &BTreeMap<String, Vec<u8>>, tags: &HashMap<String, HashSet<String>>,
    tool: &ItemStack, inputs: &[Option<ItemStack>], upgrade_slots: i32, ability_slots: i32,
    traits: &BTreeMap<String, i32>,
) -> Option<UpgradePlan> {
    let (upgrade_slots, ability_slots, occupied) = slot_state(tool, traits, upgrade_slots, ability_slots);
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
        if !ingredient_matches(tools, &tool.item, tags) { continue; }
        if recipe_type == Some("tconstruct:modifier") {
            if !matches!(id, "tconstruct:diamond" | "tconstruct:emerald" | "tconstruct:netherite" | "tconstruct:unbreakable" | "tconstruct:glowing")
                && !BONUS_SLOTS.iter().any(|(name, _, _)| *name == id) { continue; }
            if amount(tool, id) != 0 { continue; }
            if value.get("check_trait_level").and_then(Value::as_bool).unwrap_or(false)
                && traits.get(id).copied().unwrap_or(0) > 0 { continue; }
            let cost = value.pointer("/slots/upgrades").and_then(Value::as_i64).and_then(|v| i32::try_from(v).ok()).unwrap_or(0);
            let ability_cost = value.pointer("/slots/abilities").and_then(Value::as_i64).and_then(|v| i32::try_from(v).ok()).unwrap_or(0);
            if occupied + cost > upgrade_slots || used_abilities(tool) + ability_cost > ability_slots || value.get("level").and_then(Value::as_i64) != Some(1) { continue; }
            if id == "tconstruct:unbreakable" {
                let level = |name: &str, needed: i32| amount(tool, name) / needed + traits.get(name).copied().unwrap_or(0);
                if level("tconstruct:netherite", 1) < 1 || level("tconstruct:reinforced", 4) < 5 { continue; }
            }
            let Some(ingredients) = value.get("inputs").and_then(Value::as_array) else { continue; };
            let present: Vec<_> = inputs.iter().enumerate().filter_map(|(i, stack)| stack.as_ref().map(|stack| (i + 1, stack))).collect();
            let crystal = value.get("allow_crystal").and_then(Value::as_bool).unwrap_or(true)
                && present.len() == 1 && &*present[0].1.item == "tconstruct:modifier_crystal"
                && matches!(present[0].1.data().and_then(|d| d.other("minecraft:custom_data")).and_then(|d| d.get("modifier")), Some(Tag::String(value)) if value.to_string_lossy() == id);
            let mut consumed = vec![(0, 1)];
            if crystal { consumed.push((present[0].0, 1)); }
            else {
                if ingredients.len() != present.len() { continue; }
                let mut used = vec![false; present.len()];
                let mut matched = true;
                for ingredient in ingredients {
                    let found = present.iter().enumerate().position(|(i, (_, stack))| !used[i] && stack.count > 0 &&
                        ingredient_matches(ingredient, &stack.item, tags));
                    if let Some(index) = found { used[index] = true; consumed.push((present[index].0, 1)); }
                    else { matched = false; break; }
                }
                if !matched { continue; }
            }
            let bottles = if crystal { 0 } else { consumed.iter().filter(|(slot, _)| present.iter().any(|(index, stack)| index == slot && &*stack.item == "minecraft:dragon_breath")).count() as u32 };
            let leftover = (bottles > 0).then(|| ItemStack::new("minecraft:glass_bottle", bottles));
            return Some(UpgradePlan { id: id.to_owned(), amount: 1, consumed, leftover });
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
    let netherite = amount(tool, "tconstruct:netherite") > 0 || levels.get("tconstruct:netherite").copied().unwrap_or(0) > 0;
    if !diamond && !emerald && !netherite {
        let mut custom = tool.data()?.other("minecraft:custom_data")?.clone();
        let Tag::Compound(ref mut fields) = custom else { return None; };
        fields.remove(&"tconstruct:diamond_armor".into());
        fields.remove(&"tconstruct:netherite".into());
        fields.remove(&"tic_attack_conditional_multiplier".into());
        tool.data_mut().set_other("minecraft:custom_data", Some(custom));
        return Some(());
    }
    let trait_boost = 0.15 * *levels.get("tconstruct:preserved").unwrap_or(&0) as f32
        + 0.1 * *levels.get("tconstruct:ductile").unwrap_or(&0) as f32;
    let base = stats.durability + stats.durability_boost_unit * trait_boost
        + if diamond { (if armor { 250.0 } else { 500.0 }) * stats.durability_multiplier } else { 0.0 };
    let durability = (base * if emerald { 1.5 } else { 1.0 } * if netherite { 1.2 } else { 1.0 }) as i32;
    if durability < 1 { return None; }
    let mut custom = tool.data()?.other("minecraft:custom_data")?.clone();
    let Tag::Compound(ref mut fields) = custom else { return None; };
    fields.insert("tic_max_damage".into(), Tag::Int(durability));
    // Source multiplier scales eligible conditional bonuses, not the base attack value.
    if emerald { fields.insert("tic_attack_conditional_multiplier".into(), Tag::Float(1.25f32.to_bits())); }
    else { fields.remove(&"tic_attack_conditional_multiplier".into()); }
    if armor {
        if diamond { fields.insert("tic_armor".into(), Tag::Float((stats.armor + stats.armor_add_multiplier).clamp(0.0, 30.0).to_bits())); }
        if emerald || netherite {
            let ductile = *levels.get("tconstruct:ductile").unwrap_or(&0) as f32;
            let bonus = i32::from(emerald) + i32::from(netherite);
            fields.insert("tic_armor_toughness".into(), Tag::Float((stats.armor_toughness + (ductile + bonus as f32) * stats.armor_toughness_add_multiplier).clamp(0.0, 20.0).to_bits()));
        }
        if netherite { fields.insert("tic_knockback_resistance".into(), Tag::Float((stats.knockback_resistance + 0.05 * stats.knockback_resistance_add_multiplier).clamp(0.0, 1.0).to_bits())); }
    }
    if netherite { fields.insert("tconstruct:netherite".into(), Tag::Byte(1)); } else { fields.remove(&"tconstruct:netherite".into()); }
    if diamond && worn_armor { fields.insert("tconstruct:diamond_armor".into(), Tag::Byte(1)); }
    else { fields.remove(&"tconstruct:diamond_armor".into()); }
    tool.data_mut().set_other("minecraft:custom_data", Some(custom));
    tool.data_mut().set_other("minecraft:max_damage", Some(Tag::Int(durability)));
    Some(())
}

/// Rebuild additive and multiplier effects from the source baseline and saved progress.
pub fn apply_stats(
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
    let netherite = amount(tool, "tconstruct:netherite") > 0 || level("tconstruct:netherite") > 0;
    if melee { fields.insert("tic_attack_damage".into(), Tag::Float((damage * if netherite { 1.2 } else { 1.0 }).clamp(0.0, 2048.0).to_bits())); }
    if melee_weapon { fields.insert("tic_attack_speed".into(), Tag::Float(speed.to_bits())); }
    let quick = effective(tool, "tconstruct:quick_charge", 5);
    let true_level = effective(tool, "tconstruct:trueshot", 10);
    let emerald = effective(tool, "tconstruct:emerald", 1).min(1.0);
    if matches!(&*tool.item, "tconstruct:longbow" | "tconstruct:crossbow") {
        let light = level("tconstruct:lightweight").max(0) as f32;
        let feather = level("tconstruct:featherweight").max(0) as f32;
        let heavy = level("tconstruct:heavy").max(0) as f32;
        let vintage = level("tconstruct:vintage").max(0) as f32;
        let crystal = level("tconstruct:crystalbound").max(0) as f32;
        let entangled = if level("tconstruct:entangled") > 0 { 0.15 } else { 0.0 };
        let draw = ((stats.draw_speed + stats.draw_speed_add_multiplier * (0.03 * light + 0.05 * feather + entangled))
            * (1.0 + if &*tool.item == "tconstruct:crossbow" { 0.25 * quick } else { 0.0 })
            * (1.0 - 0.1 * vintage)).clamp(0.0, 1024.0);
        let velocity = ((stats.velocity + stats.velocity_add_multiplier * (0.03 * light + 0.1 * crystal))
            * if netherite { 1.1 } else { 1.0 }).clamp(0.0, 1024.0);
        let accuracy = (stats.accuracy + stats.accuracy_add_multiplier * (0.1 * (true_level + emerald + heavy) + 0.05 * feather)).clamp(0.1, 1.0);
        let projectile_damage = (stats.projectile_damage + stats.projectile_damage_add_multiplier
            * (0.5 * effective(tool, "tconstruct:diamond", 1).min(1.0) + 0.25 * level("tconstruct:ductile").max(0) as f32)).clamp(0.0, 1024.0);
        fields.insert("tic_draw_speed".into(), Tag::Float(draw.to_bits()));
        fields.insert("tic_velocity".into(), Tag::Float(velocity.to_bits()));
        fields.insert("tic_dwarven".into(), Tag::Int(level("tconstruct:dwarven").max(0)));
        fields.insert("tic_dwarven_velocity_multiplier".into(), Tag::Float((stats.velocity_add_multiplier * if netherite { 1.1 } else { 1.0 }).to_bits()));
        fields.insert("tic_accuracy".into(), Tag::Float(accuracy.to_bits()));
        fields.insert("tic_projectile_damage".into(), Tag::Float(projectile_damage.to_bits()));
        fields.insert("tic_crystalbound_angle".into(), Tag::Int(crystal as i32));
    }
    tool.data_mut().set_other("minecraft:custom_data", Some(custom));
    Some(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn material_change_rechecks_used_slots_against_new_traits() {
        let mut tool=ItemStack::new("tconstruct:pickaxe",1);
        tool.data_mut().set_other("minecraft:custom_data",Some(Tag::compound([("tic_upgrades",Tag::compound([
            ("tconstruct:reinforced",Tag::Int(12)),("tconstruct:diamond",Tag::Int(1)),
        ]))])));
        assert!(valid_slots(&tool,&BTreeMap::from([("tconstruct:enhanced".into(),1)]),3,1));
        assert!(!valid_slots(&tool,&BTreeMap::new(),3,1));
        assert!(valid_slots(&tool,&BTreeMap::new(),4,1));
    }

    #[test]
    fn overforced_partial_slots_and_crystal_round_trip() {
        let recipe = br#"{"type":"tconstruct:incremental_modifier","allow_crystal":true,"amount_per_item":1,"needed_per_level":4,"input":{"item":"tconstruct:slimesteel_reinforcement"},"result":"tconstruct:overforced","level":{"max":5},"slots":{"upgrades":1},"tools":{"tag":"tconstruct:modifiable/durability"}}"#.to_vec();
        let recipes = BTreeMap::from([("overforced".into(), recipe)]);
        let tags = HashMap::from([("tconstruct:modifiable/durability".into(), HashSet::from(["tconstruct:pickaxe".into()])), ("tconstruct:modifiable".into(), HashSet::from(["tconstruct:pickaxe".into()]))]);
        let mut tool = ItemStack::new("tconstruct:pickaxe", 1);
        let set = |tool: &mut ItemStack, amount| tool.data_mut().set_other("minecraft:custom_data", Some(Tag::compound([("tic_upgrades", Tag::compound([("tconstruct:overforced", Tag::Int(amount))]))])));
        let ingredients = [Some(ItemStack::new("tconstruct:slimesteel_reinforcement", 9))];
        assert!(plan(&recipes, &tags, &tool, &ingredients, 0).is_none());
        let first = plan(&recipes, &tags, &tool, &ingredients, 1).unwrap();
        assert_eq!(first.amount, 4);
        assert_eq!(first.consumed, [(0, 1), (1, 4)]);
        set(&mut tool, 7);
        let extracted = extract_upgrade(&tags, &tool, &[Some(ItemStack::new("minecraft:wet_sponge", 1)), Some(ItemStack::new("tconstruct:sky_slime_crystal", 1))], "tconstruct:overforced").unwrap();
        assert_eq!(extracted.upgrade.amount, 3);
        set(&mut tool, 3);
        assert_eq!(plan(&recipes, &tags, &tool, &[extracted.upgrade.leftover], 2).unwrap().amount, 7);
        set(&mut tool, 20);
        assert!(plan(&recipes, &tags, &tool, &ingredients, 10).is_none());
    }

    #[test]
    fn glowing_recipe_respects_ability_capacity_target_and_trait() {
        let recipe = br#"{"type":"tconstruct:modifier","allow_crystal":true,"check_trait_level":true,"inputs":[{"item":"minecraft:glowstone"},{"item":"minecraft:daylight_detector"},{"item":"minecraft:shroomlight"}],"level":1,"result":"tconstruct:glowing","slots":{"abilities":1},"tools":{"type":"forge:intersection","children":[{"tag":"tconstruct:modifiable/durability"},[{"tag":"tconstruct:modifiable/interactable"},{"tag":"tconstruct:modifiable/armor/boots"}]]}}"#.to_vec();
        let recipes = BTreeMap::from([("glowing".into(),recipe)]);
        let tags = HashMap::from([
            ("tconstruct:modifiable/durability".into(), HashSet::from(["tconstruct:pickaxe".into(),"tconstruct:plate_boots".into(),"tconstruct:helmet".into()])),
            ("tconstruct:modifiable/interactable".into(), HashSet::from(["tconstruct:pickaxe".into()])),
            ("tconstruct:modifiable/armor/boots".into(), HashSet::from(["tconstruct:plate_boots".into()])),
        ]);
        let ingredients = ["minecraft:glowstone","minecraft:daylight_detector","minecraft:shroomlight"].map(|id| Some(ItemStack::new(id,1)));
        let pick=ItemStack::new("tconstruct:pickaxe",1);
        assert_eq!(plan_with_traits(&recipes,&tags,&pick,&ingredients,0,1,&BTreeMap::new()).unwrap().id,"tconstruct:glowing");
        let mut crystal = ItemStack::new("tconstruct:modifier_crystal",1);
        crystal.data_mut().set_other("minecraft:custom_data",Some(Tag::compound([("modifier",Tag::String("tconstruct:glowing".into()))])));
        assert!(plan_with_traits(&recipes,&tags,&pick,&[Some(crystal)],0,1,&BTreeMap::new()).is_some());
        assert!(plan_with_traits(&recipes,&tags,&pick,&ingredients,10,0,&BTreeMap::new()).is_none());
        assert!(plan_with_traits(&recipes,&tags,&ItemStack::new("tconstruct:helmet",1),&ingredients,0,1,&BTreeMap::new()).is_none());
        assert!(plan_with_traits(&recipes,&tags,&ItemStack::new("tconstruct:plate_boots",1),&ingredients,0,1,&BTreeMap::new()).is_some());
        assert!(plan_with_traits(&recipes,&tags,&pick,&ingredients,0,1,&BTreeMap::from([("tconstruct:glowing".into(),1)])).is_none());
        let mut occupied=pick.clone();occupied.data_mut().set_other("minecraft:custom_data",Some(Tag::compound([("tic_upgrades",Tag::compound([("tconstruct:unbreakable",Tag::Int(1))]))])));
        assert!(plan_with_traits(&recipes,&tags,&occupied,&ingredients,0,1,&BTreeMap::new()).is_none());
        occupied.data_mut().set_other("minecraft:custom_data",Some(Tag::compound([("tic_upgrades",Tag::compound([("tconstruct:unbreakable",Tag::Int(1)),("tconstruct:glowing",Tag::Int(1))]))])));
        assert!(!valid_slots(&occupied,&BTreeMap::new(),0,1));
        assert!(valid_slots(&occupied,&BTreeMap::new(),0,2));
    }

    #[test]
    fn netherite_and_unbreakable_recipes_and_dependencies() {
        let netherite = br#"{"type":"tconstruct:modifier","allow_crystal":true,"inputs":[{"item":"minecraft:netherite_upgrade_smithing_template"},{"tag":"forge:ingots/netherite"}],"level":1,"result":"tconstruct:netherite","slots":{"upgrades":1},"tools":{"tag":"tconstruct:modifiable/durability"}}"#.to_vec();
        let unbreakable = br#"{"type":"tconstruct:modifier","allow_crystal":true,"check_trait_level":true,"inputs":[{"item":"minecraft:shulker_shell"},{"item":"minecraft:dragon_breath"},{"item":"minecraft:shulker_shell"},{"tag":"forge:ingots/netherite"},{"tag":"forge:ingots/netherite"}],"level":1,"result":"tconstruct:unbreakable","slots":{"abilities":1},"tools":{"tag":"tconstruct:modifiable/durability"}}"#.to_vec();
        let recipes = BTreeMap::from([("netherite".into(), netherite), ("unbreakable".into(), unbreakable)]);
        let tags = HashMap::from([("tconstruct:modifiable/durability".into(), HashSet::from(["tconstruct:pickaxe".into()])), ("tconstruct:modifiable".into(), HashSet::from(["tconstruct:pickaxe".into()])), ("forge:ingots/netherite".into(), HashSet::from(["minecraft:netherite_ingot".into()]))]);
        let mut tool = ItemStack::new("tconstruct:pickaxe", 1);
        let recipe = [Some(ItemStack::new("minecraft:netherite_ingot", 2)), Some(ItemStack::new("minecraft:netherite_upgrade_smithing_template", 1))];
        assert!(plan_with_traits(&recipes,&tags,&tool,&recipe,0,1,&BTreeMap::new()).is_none());
        let applied = plan_with_traits(&recipes,&tags,&tool,&recipe,6,1,&BTreeMap::new()).unwrap();
        assert_eq!(applied.id,"tconstruct:netherite");
        assert_eq!(applied.consumed, vec![(0,1),(2,1),(1,1)]);
        let set = |tool: &mut ItemStack, amount| tool.data_mut().set_other("minecraft:custom_data",Some(Tag::compound([("tic_upgrades",Tag::compound([("tconstruct:netherite",Tag::Int(1)),("tconstruct:reinforced",Tag::Int(amount))]))])));
        let five = [Some(ItemStack::new("minecraft:shulker_shell",1)),Some(ItemStack::new("minecraft:dragon_breath",1)),Some(ItemStack::new("minecraft:shulker_shell",1)),Some(ItemStack::new("minecraft:netherite_ingot",1)),Some(ItemStack::new("minecraft:netherite_ingot",1))];
        set(&mut tool,19);
        assert!(plan_with_traits(&recipes,&tags,&tool,&five,6,1,&BTreeMap::new()).is_none());
        set(&mut tool,20);
        assert!(plan_with_traits(&recipes,&tags,&tool,&five[..3],6,1,&BTreeMap::new()).is_none());
        assert!(plan_with_traits(&recipes,&tags,&tool,&five,6,0,&BTreeMap::new()).is_none());
        let planned = plan_with_traits(&recipes,&tags,&tool,&five,6,1,&BTreeMap::new()).unwrap();
        assert_eq!(planned.id,"tconstruct:unbreakable");
        assert_eq!(&*planned.leftover.unwrap().item, "minecraft:glass_bottle");
        let mut crystal=ItemStack::new("tconstruct:modifier_crystal",1);
        crystal.data_mut().set_other("minecraft:custom_data",Some(Tag::compound([("modifier",Tag::String("tconstruct:unbreakable".into()))])));
        assert_eq!(plan_with_traits(&recipes,&tags,&tool,&[Some(crystal)],6,1,&BTreeMap::new()).unwrap().id,"tconstruct:unbreakable");
        // A two-upgrade, zero-ability base can reach the recipe through the
        // source bonus modifiers instead of changing its intrinsic slots.
        let bonuses = BTreeMap::from([("tconstruct:writable".into(),1), ("tconstruct:harmonious".into(),1), ("tconstruct:recapitated".into(),1), ("tconstruct:forecast".into(),1), ("tconstruct:draconic".into(),1)]);
        assert_eq!(plan_with_traits(&recipes,&tags,&tool,&five,2,0,&bonuses).unwrap().id,"tconstruct:unbreakable");
        tool.data_mut().set_other("minecraft:custom_data",Some(Tag::compound([("tic_upgrades",Tag::compound([("tconstruct:netherite",Tag::Int(1)),("tconstruct:reinforced",Tag::Int(20)),("tconstruct:unbreakable",Tag::Int(1))]))])));
        let inputs=[Some(ItemStack::new("minecraft:wet_sponge",1)),Some(ItemStack::new("tconstruct:sky_slime_crystal",1))];
        assert!(extract_upgrade(&tags,&tool,&inputs,"tconstruct:reinforced").is_none());
        assert!(extract_upgrade(&tags,&tool,&inputs,"tconstruct:netherite").is_none());
        assert!(extract_upgrade(&tags,&tool,&inputs,"tconstruct:unbreakable").is_some());
        assert!(valid_unbreakable(&tool,&BTreeMap::new()));
    }

    #[test]
    fn source_bonus_slots_make_armor_and_bow_prerequisites_reachable() {
        let writable = br#"{"type":"tconstruct:modifier","allow_crystal":true,"inputs":[{"item":"minecraft:writable_book"}],"level":1,"result":"tconstruct:writable","tools":{"tag":"tconstruct:modifiable/bonus_slots"}}"#.to_vec();
        let draconic = br#"{"type":"tconstruct:modifier","allow_crystal":true,"inputs":[{"item":"minecraft:dragon_head"}],"level":1,"result":"tconstruct:draconic","tools":{"type":"forge:difference","base":{"tag":"tconstruct:modifiable/bonus_slots"},"subtracted":{"tag":"tconstruct:modifiable/skulls"}}}"#.to_vec();
        let reinforced = br#"{"type":"tconstruct:incremental_modifier","amount_per_item":1,"needed_per_level":4,"input":{"item":"tconstruct:emerald_reinforcement"},"result":"tconstruct:reinforced","level":{"max":5},"slots":{"upgrades":1},"tools":{"tag":"tconstruct:modifiable/durability"}}"#.to_vec();
        let recipes=BTreeMap::from([("writable".into(),writable),("draconic".into(),draconic),("reinforced".into(),reinforced)]);
        let tags=HashMap::from([("tconstruct:modifiable/bonus_slots".into(),HashSet::from(["tconstruct:plate_chestplate".into()])),("tconstruct:modifiable/durability".into(),HashSet::from(["tconstruct:plate_chestplate".into()]))]);
        let mut armor=ItemStack::new("tconstruct:plate_chestplate",1);
        armor.data_mut().set_other("minecraft:custom_data",Some(Tag::compound([("tic_upgrades",Tag::compound([("tconstruct:reinforced",Tag::Int(8))]))])));
        assert_eq!(plan_with_traits(&recipes,&tags,&armor,&[Some(ItemStack::new("minecraft:writable_book",1))],2,0,&BTreeMap::new()).unwrap().id,"tconstruct:writable");
        assert_eq!(plan_with_traits(&recipes,&tags,&armor,&[Some(ItemStack::new("minecraft:dragon_head",1))],2,0,&BTreeMap::new()).unwrap().id,"tconstruct:draconic");
        let extra=BTreeMap::from([("tconstruct:writable".into(),1)]);
        assert_eq!(plan_with_traits(&recipes,&tags,&armor,&[Some(ItemStack::new("tconstruct:emerald_reinforcement",1))],2,0,&extra).unwrap().amount,9);
        let enhanced=BTreeMap::from([("tconstruct:enhanced".into(),1)]);
        assert_eq!(plan_with_traits(&recipes,&tags,&armor,&[Some(ItemStack::new("tconstruct:emerald_reinforcement",1))],2,0,&enhanced).unwrap().amount,9);
        assert!(plan_with_traits(&recipes,&tags,&armor,&[Some(ItemStack::new("tconstruct:emerald_reinforcement",1))],2,0,&BTreeMap::new()).is_none());
        let difference=serde_json::json!({"type":"forge:difference","base":{"tag":"forge:heads"},"subtracted":{"item":"minecraft:dragon_head"}});
        let skulls=HashMap::from([("forge:heads".into(),HashSet::from(["minecraft:dragon_head".into(),"minecraft:skeleton_skull".into()]))]);
        assert!(ingredient_matches(&difference,"minecraft:skeleton_skull",&skulls));
        assert!(!ingredient_matches(&difference,"minecraft:dragon_head",&skulls));
    }

    #[test]
    fn netherite_stats_rebuild_from_original_base_with_gems() {
        let definition = br#"{"modules":[{"type":"tconstruct:base_stats","stats":{"tconstruct:durability":100,"tconstruct:attack_damage":4,"tconstruct:armor_toughness":2,"tconstruct:knockback_resistance":0.1,"tconstruct:velocity":1.0}}]}"#;
        let stats = super::super::tconstruct_source_station_stats::base_stats(definition,&[]).unwrap();
        let mut tool=ItemStack::new("tconstruct:crossbow",1);
        tool.data_mut().set_other("minecraft:custom_data",Some(Tag::compound([("tic_upgrades",Tag::compound([("tconstruct:netherite",Tag::Int(1)),("tconstruct:emerald",Tag::Int(1)),("tconstruct:diamond",Tag::Int(1))]))])));
        for _ in 0..2 {
            apply_gem_durability_and_armor(&mut tool,&stats,&BTreeMap::new(),true,true).unwrap();
            apply_stats(&mut tool,&stats,&BTreeMap::new(),true,true).unwrap();
            let data=tool.data().unwrap().other("minecraft:custom_data").unwrap();
            assert!(matches!(data.get("tic_max_damage"),Some(Tag::Int(630))));
            let get=|name| match data.get(name) { Some(Tag::Float(v)) => f32::from_bits(*v), _ => panic!("missing {name}") };
            assert!((get("tic_armor_toughness")-4.0).abs()<0.0001);
            assert!((get("tic_knockback_resistance")-0.15).abs()<0.0001);
            assert!((get("tic_attack_damage")-5.4).abs()<0.0001);
            assert!((get("tic_velocity")-1.1).abs()<0.0001);
        }
    }

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
        assert_eq!(selection_index("tconstruct:glowing").and_then(selection_id), Some("tconstruct:glowing"));
        assert!(plan(&recipes, &tags, &ItemStack::new("tconstruct:war_pick", 1), &[Some(ItemStack::new("minecraft:magma_cream", 1))], 3).is_none());
        assert!(plan(&recipes, &tags, &ItemStack::new("tconstruct:javelin", 1), &[Some(ItemStack::new("minecraft:target", 1))], 3).is_none());
    }

    #[test]
    fn ranged_rebuild_adds_and_removes_dwarven_level() {
        let definition = br#"{"modules":[{"type":"tconstruct:multiply_stats","multipliers":{"tconstruct:velocity":2.0}}]}"#;
        let stats = super::super::tconstruct_source_station_stats::base_stats(definition, &[]).unwrap();
        let mut bow = ItemStack::new("tconstruct:longbow", 1);
        bow.data_mut().set_other("minecraft:custom_data", Some(Tag::compound([("tic_upgrades", Tag::compound([("test:empty", Tag::Int(0))]))])));
        let levels = BTreeMap::from([("tconstruct:dwarven".to_owned(), 1)]);
        apply_stats(&mut bow, &stats, &levels, false, false).unwrap();
        let custom = bow.data().unwrap().other("minecraft:custom_data").unwrap();
        assert!(matches!(custom.get("tic_dwarven"), Some(Tag::Int(1))));
        let Some(Tag::Float(multiplier)) = custom.get("tic_dwarven_velocity_multiplier") else { panic!() };
        assert!((f32::from_bits(*multiplier) - stats.velocity_add_multiplier).abs() < 0.00001);
        apply_stats(&mut bow, &stats, &BTreeMap::new(), false, false).unwrap();
        assert!(matches!(bow.data().unwrap().other("minecraft:custom_data").unwrap().get("tic_dwarven"), Some(Tag::Int(0))));
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
