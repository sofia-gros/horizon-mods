// Supported source trait formulas. Other modifier effects are not interpreted here.
pub(super) fn repair_factor(cultivated: i32, preserved: i32) -> f32 {
    (1.0 + cultivated as f32 * 0.5) * (1.0 + preserved as f32 * 0.15)
}

/// Apply known repair hooks to the supplied amount in source modifier order.
/// Ambiguous upgrade or trait order fails before consumption.
pub(super) fn ordered_known_repair_amount(stack: &horizon_items::items::ItemStack,
    traits: &std::collections::BTreeMap<String, i32>, trait_order: &[String], initial: f32) -> Option<f32> {
    use horizon_world::nbt::Tag;
    const IDS: [&str; 4] = ["tconstruct:emerald", "tconstruct:cultivated", "tconstruct:preserved", "tconstruct:dense"];
    let custom = stack.data()?.other("minecraft:custom_data")?;
    let upgrades = custom.get("tic_upgrades");
    if upgrades.is_some_and(|tag| !matches!(tag, Tag::Compound(_))) || !initial.is_finite() { return None; }
    let mut active = Vec::new();
    let mut values = std::collections::BTreeMap::new();
    for id in IDS {
        let upgrade = match upgrades.and_then(|u| u.get(id)) { None => 0, Some(Tag::Int(level)) if *level >= 0 => *level, _ => return None };
        let trait_level = traits.get(id).copied().unwrap_or(0);
        if trait_level < 0 { return None; }
        let level = trait_level.checked_add(upgrade)?;
        if upgrade > 0 { active.push(id); }
        values.insert(id, level);
    }
    let mut order: Vec<&str> = Vec::new();
    if active.len() == 1 { order.push(active[0]); }
    else if active.len() > 1 {
        let Some(Tag::List(saved)) = custom.get("tic_upgrade_order") else { return None; };
        for tag in saved {
            let Tag::String(id) = tag else { return None; };
            let id = id.to_string_lossy();
            if let Some(&known) = active.iter().find(|known| **known == id.as_str()) {
                if order.contains(&known) { return None; }
                order.push(known);
            }
        }
        if order.len() != active.len() { return None; }
    }
    for id in trait_order {
        if let Some(&known) = IDS.iter().find(|known| **known == id.as_str()) {
            if traits.get(known).copied().unwrap_or(0) > 0 && !order.contains(&known) { order.push(known); }
        }
    }
    if IDS.iter().any(|id| traits.get(*id).copied().unwrap_or(0) > 0 && !order.contains(id)) { return None; }
    let mut amount = initial;
    for id in order {
        let level = values[id];
        let multiplier = match id {
            "tconstruct:emerald" if level > 0 => 1.5,
            "tconstruct:cultivated" => 1.0 + 0.5 * level as f32,
            "tconstruct:preserved" => 1.0 + 0.15 * level as f32,
            "tconstruct:dense" => dense_repair_factor(level),
            _ => 1.0,
        };
        amount *= multiplier;
        if !amount.is_finite() { return None; }
        if amount <= 0.0 { return Some(0.0); }
    }
    Some(amount)
}

/// Supported casting part-swap repair hooks. More than one active factor is
/// refused until the casting path supports full source hook ordering.
pub(super) fn limited_casting_repair_factor(traits: &std::collections::BTreeMap<String, i32>, emerald_upgrade: bool) -> Option<f32> {
    if traits.keys().any(|id| !matches!(id.as_str(), "tconstruct:preserved" | "tconstruct:ductile" | "tconstruct:cultivated" | "tconstruct:dense" | "tconstruct:emerald")) { return None; }
    let level = |id: &str| traits.get(id).copied().unwrap_or(0);
    let emerald = emerald_upgrade || level("tconstruct:emerald") > 0;
    if [level("tconstruct:preserved") > 0, level("tconstruct:cultivated") > 0,
        level("tconstruct:dense") > 0, emerald].into_iter().filter(|v| *v).count() > 1 { return None; }
    Some(repair_factor(level("tconstruct:cultivated"), level("tconstruct:preserved"))
        * dense_repair_factor(level("tconstruct:dense")) * if emerald { 1.5 } else { 1.0 })
}

/// Ordered known repair hooks for supported casting part swaps. Unknown traits
/// and ambiguous order remain closed before the transaction proceeds.
pub(super) fn casting_repair_amount(stack: &horizon_items::items::ItemStack,
    traits: &std::collections::BTreeMap<String, i32>, order: &[String], initial: f32) -> Option<f32> {
    use horizon_world::nbt::Tag;
    if !initial.is_finite() { return None; }
    let upgrades = stack.data()?.other("minecraft:custom_data")?.get("tic_upgrades");
    if upgrades.is_some_and(|tag| !matches!(tag, Tag::Compound(_))) { return None; }
    let emerald_upgrade = match upgrades.and_then(|u| u.get("tconstruct:emerald")) {
        None | Some(Tag::Int(0)) => false, Some(Tag::Int(level)) if *level > 0 => true, _ => return None,
    };
    if traits.keys().any(|id| !matches!(id.as_str(), "tconstruct:preserved" | "tconstruct:ductile" | "tconstruct:cultivated" | "tconstruct:dense" | "tconstruct:emerald")) { return None; }
    let level = |id: &str| traits.get(id).copied().unwrap_or(0);
    let emerald = emerald_upgrade || level("tconstruct:emerald") > 0;
    let active = [level("tconstruct:preserved") > 0, level("tconstruct:cultivated") > 0,
        level("tconstruct:dense") > 0, emerald].into_iter().filter(|v| *v).count();
    if level("tconstruct:emerald") > 0 || active > 1 {
        ordered_known_repair_amount(stack, traits, order, initial)
    } else {
        Some(initial * limited_casting_repair_factor(traits, emerald_upgrade)?)
    }
}

/// Dense repair follows the two level ranges in the original repair module.
pub(super) fn dense_repair_factor(level: i32) -> f32 {
    match level {
        i32::MIN..=0 => 1.0,
        1..=5 => 1.0 - 0.025 * level as f32 * (11 - level) as f32,
        // Source fallback evaluates 1 + (flat + per-level), not the
        // algebraically equivalent 0.5 - 0.05 * level; f32 rounding differs.
        _ => (1.0 + (-0.5 + -0.05 * level as f32)).max(0.0),
    }
}

/// Chance that one point of tool damage is prevented by Dense.
pub(super) fn dense_reduction_probability(level: i32) -> f32 {
    if level <= 0 { 0.0 } else { 1.0 - 1.5f32.powi(-level) }
}

/// Apply the source independent cancellation roll to each point of damage.
pub(super) fn dense_reduce(mut loss: u32, level: i32, mut next: impl FnMut() -> f32) -> u32 {
    let chance = dense_reduction_probability(level);
    if chance <= 0.0 { return loss; }
    for _ in 0..loss { if next() < chance { loss -= 1; } }
    loss
}

/// Source secondary-durability causes bypass the primary clamp and are
/// suppressed by Tanned's second, higher-priority range module.
pub(super) fn tanned_wear(loss: u32, level: i32, cause: Option<&str>) -> u32 {
    if level <= 0 { return loss; }
    if matches!(cause, Some("tconstruct:protection" | "tconstruct:thorns" | "tconstruct:fiery" | "tconstruct:freezing" | "tconstruct:springy" | "tconstruct:pierce" | "tconstruct:venom" | "tconstruct:conductive" | "tconstruct:shock" | "tconstruct:necrotic" | "tconstruct:restore" | "tconstruct:enderporting")) { 0 }
    else { loss.min(1) }
}

/// Stringy's original material-repair module contributes 140 per level.
pub(super) fn stringy_repair(material: &str, level: i32) -> f32 {
    if material.split('#').next() == Some("tconstruct:string") { 140.0 * level.max(0) as f32 } else { 0.0 }
}

/// Source Reinforced chance uses the effective incremental level.
pub(super) fn reinforced_probability(amount: i32) -> f32 {
    let level = amount.max(0) as f32 / 4.0;
    if level <= 5.0 { (0.025 * level * (11.0 - level)).clamp(0.0, 1.0) }
    else { (0.5 + 0.05 * level).clamp(0.0, 1.0) }
}

/// The pinned source bypass tag currently contains Glowing. Modifier-driven
/// wear must pass its cause ID; ordinary use passes None.
pub(super) fn reinforced_cause_bypass(cause: Option<&str>) -> bool {
    cause == Some("tconstruct:glowing")
}

/// Ordinary wear applies Dense before Reinforced (equal source priority, ID order).
/// Explicit bypass applies only to Reinforced, as with the source cause tag.
pub(super) fn reinforced_wear(loss: u32, dense: i32, amount: i32, unbreakable: bool,
    cause: Option<&str>, mut next: impl FnMut() -> f32) -> u32 {
    // Unbreakable runs before Dense, but both it and Reinforced respect the
    // source bypass tag. Dense still runs for a bypassed cause.
    if unbreakable && !reinforced_cause_bypass(cause) { return 0; }
    let loss = dense_reduce(loss, dense, &mut next);
    if unbreakable || reinforced_cause_bypass(cause) { return loss; }
    let chance = reinforced_probability(amount);
    if chance <= 0.0 { return loss; }
    if chance >= 1.0 { return 0; }
    (0..loss).filter(|_| next() >= chance).count() as u32
}

pub(super) fn durability_boost(preserved: i32, ductile: i32) -> f32 {
    preserved as f32 * 0.15 + ductile as f32 * 0.1
}

pub(super) fn boosted_durability(base: f32, boost_unit: f32, preserved: i32, ductile: i32) -> i32 {
    (base + boost_unit * durability_boost(preserved, ductile)) as i32
}

pub(super) fn wear_damage_bonus(
    damage: f32,
    lost: u32,
    durability_multiplier: f32,
    jagged: i32,
    stonebound: i32,
) -> f32 {
    let wear = (lost as f32 / durability_multiplier).sqrt() * 0.005;
    damage * ((1.0 + jagged as f32 * wear) * (1.0 - stonebound as f32 * wear) - 1.0)
}

/// Apply the supported source stat traits to an assembled or Creative stack.
pub fn apply_stat_traits(
    stack: &mut horizon_items::items::ItemStack,
    stats: &super::tconstruct_source_station_stats::BaseStats,
    levels: &std::collections::BTreeMap<String, i32>,
) -> Option<()> {
    let preserved = levels.get("tconstruct:preserved").copied().unwrap_or(0);
    let ductile = levels.get("tconstruct:ductile").copied().unwrap_or(0);
    let overlord = levels.get("tconstruct:overlord").copied().unwrap_or(0).max(0);
    if preserved <= 0 && ductile <= 0 && overlord <= 0 {
        return Some(());
    }
    let durability = (stats.durability_base_raw * (1.0 + stats.durability_percent_raw) * ordered_durability_multiplier(stack, stats, levels, true)) as i32;
    let durability = if overlord > 0 { durability.max(1) } else { durability };
    if durability < 1 {
        return None;
    }
    let mut custom = stack.data()?.other("minecraft:custom_data")?.clone();
    let horizon_world::nbt::Tag::Compound(ref mut fields) = custom else {
        return None;
    };
    fields.insert(
        "tic_max_damage".into(),
        horizon_world::nbt::Tag::Int(durability),
    );
    if ductile > 0 {
        let attack = stats.attack_damage + stats.attack_damage_boost_unit * 0.05 * ductile as f32;
        let toughness = (stats.armor_toughness + ductile as f32 * stats.armor_toughness_add_multiplier).clamp(0.0, 20.0);
        fields.insert(
            "tic_attack_damage".into(),
            horizon_world::nbt::Tag::Float(attack.to_bits()),
        );
        fields.insert(
            "tic_armor_toughness".into(),
            horizon_world::nbt::Tag::Float(toughness.to_bits()),
        );
    }
    stack
        .data_mut()
        .set_other("minecraft:custom_data", Some(custom));
    stack.data_mut().set_other(
        "minecraft:max_damage",
        Some(horizon_world::nbt::Tag::Int(durability)),
    );
    Some(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ductile_and_overlord_multiply_without_erasing_other_stats() {
        use horizon_items::items::ItemStack;
        use horizon_world::nbt::Tag;
        let definition = br#"{"modules":[{"type":"tconstruct:base_stats","stats":{"tconstruct:durability":100,"tconstruct:attack_damage":10,"tconstruct:armor_toughness":2}}]}"#;
        let stats = super::super::tconstruct_source_station_stats::base_stats(definition, &[]).unwrap();
        let mut stack = ItemStack::new("tconstruct:pickaxe", 1);
        stack.data_mut().set_other("minecraft:custom_data", Some(Tag::compound([("tic_max_damage", Tag::Int(100))])));
        let levels = [("tconstruct:ductile".to_owned(), 3), ("tconstruct:overlord".to_owned(), 2)].into_iter().collect();
        apply_stat_traits(&mut stack, &stats, &levels).unwrap();
        let custom = stack.data().unwrap().other("minecraft:custom_data").unwrap();
        // Source: 100 * (1 + 0.1 * 3) * (1 - 0.15 * 2).
        assert!(matches!(custom.get("tic_max_damage"), Some(Tag::Int(91))));
        assert!(matches!(custom.get("tic_attack_damage"), Some(Tag::Float(bits)) if (f32::from_bits(*bits) - 11.5).abs() < 0.001));
        assert!(matches!(custom.get("tic_armor_toughness"), Some(Tag::Float(bits)) if (f32::from_bits(*bits) - 5.0).abs() < 0.001));
    }
    #[test]
    fn tanned_primary_and_secondary_cause_ranges_precede_shield() {
        assert_eq!(tanned_wear(7, 0, None), 7);
        assert_eq!(tanned_wear(7, 1, None), 1);
        assert_eq!(tanned_wear(0, 1, None), 0);
        assert_eq!(tanned_wear(7, 1, Some("tconstruct:glowing")), 1);
        assert_eq!(tanned_wear(7, 1, Some("tconstruct:protection")), 0);
        assert_eq!(tanned_wear(7, 1, Some("tconstruct:thorns")), 0);
    }
    #[test]
    fn stringy_provides_only_source_string_repair() {
        assert_eq!(stringy_repair("tconstruct:string",2),280.0);
        assert_eq!(stringy_repair("tconstruct:string#variant",1),140.0);
        assert_eq!(stringy_repair("tconstruct:wood",1),0.0);
    }
    #[test]
    fn durability_boost_updates_both_components_and_preserves_stack_data() {
        use horizon_items::items::ItemStack;
        use horizon_world::nbt::Tag;
        let definition = br#"{"modules":[{"type":"tconstruct:part_stats","parts":["tconstruct:pick_head","tconstruct:tool_handle"]}]}"#;
        let head = serde_json::json!({"tconstruct:head":{"durability":85}})
            .as_object()
            .unwrap()
            .clone();
        let handle = serde_json::json!({"tconstruct:handle":{"durability":0}})
            .as_object()
            .unwrap()
            .clone();
        let stats =
            super::super::tconstruct_source_station_stats::base_stats(definition, &[head, handle])
                .unwrap();
        let mut stack = ItemStack::new("tconstruct:pickaxe", 1);
        stack.damage = 10;
        stack.data_mut().set_other(
            "minecraft:custom_data",
            Some(Tag::compound([
                ("tic_max_damage", Tag::Int(85)),
                ("keep", Tag::Int(42)),
            ])),
        );
        stack
            .data_mut()
            .set_other("minecraft:max_damage", Some(Tag::Int(85)));
        let levels = [("tconstruct:preserved".to_owned(), 1)]
            .into_iter()
            .collect();
        apply_stat_traits(&mut stack, &stats, &levels).unwrap();
        assert_eq!(stack.damage, 10);
        assert!(matches!(
            stack.data().unwrap().other("minecraft:max_damage"),
            Some(Tag::Int(97))
        ));
        let custom = stack
            .data()
            .unwrap()
            .other("minecraft:custom_data")
            .unwrap();
        assert!(matches!(custom.get("tic_max_damage"), Some(Tag::Int(97))));
        assert!(matches!(custom.get("keep"), Some(Tag::Int(42))));
    }

    #[test]
    fn ductile_updates_attack_and_toughness_once_from_base_stats() {
        use horizon_items::items::ItemStack;
        use horizon_world::nbt::Tag;
        let definition = br#"{"modules":[{"type":"tconstruct:part_stats","parts":["tconstruct:pick_head","tconstruct:tool_handle"]}]}"#;
        let head = serde_json::json!({"tconstruct:head":{"durability":100,"melee_attack":4}})
            .as_object()
            .unwrap()
            .clone();
        let handle = serde_json::json!({"tconstruct:handle":{"durability":0,"melee_damage":0.25}})
            .as_object()
            .unwrap()
            .clone();
        let stats =
            super::super::tconstruct_source_station_stats::base_stats(definition, &[head, handle])
                .unwrap();
        let mut stack = ItemStack::new("tconstruct:pickaxe", 1);
        stack.data_mut().set_other(
            "minecraft:custom_data",
            Some(Tag::compound([("tic_max_damage", Tag::Int(100))])),
        );
        let levels = [("tconstruct:ductile".to_owned(), 2)].into_iter().collect();
        apply_stat_traits(&mut stack, &stats, &levels).unwrap();
        apply_stat_traits(&mut stack, &stats, &levels).unwrap();
        let custom = stack
            .data()
            .unwrap()
            .other("minecraft:custom_data")
            .unwrap();
        let Some(Tag::Float(attack)) = custom.get("tic_attack_damage") else {
            panic!("missing attack")
        };
        let Some(Tag::Float(toughness)) = custom.get("tic_armor_toughness") else {
            panic!("missing toughness")
        };
        assert!((f32::from_bits(*attack) - 5.4).abs() < 0.00001);
        assert_eq!(f32::from_bits(*toughness), 2.0);
        assert!(matches!(custom.get("tic_max_damage"), Some(Tag::Int(120))));
    }

    #[test]
    fn source_repair_and_wear_effects_scale_with_levels() {
        assert_eq!(repair_factor(0, 0), 1.0);
        assert_eq!(dense_repair_factor(0), 1.0);
        assert!((dense_repair_factor(1) - 0.75).abs() < 0.00001);
        assert!((dense_repair_factor(5) - 0.25).abs() < 0.00001);
        assert!((dense_repair_factor(6) - 0.20).abs() < 0.00001);
        assert_eq!(dense_repair_factor(10), 0.0);
        assert!((dense_reduction_probability(1) - 1.0 / 3.0).abs() < 0.00001);
        assert!((dense_reduction_probability(2) - 5.0 / 9.0).abs() < 0.00001);
        let mut rolls = [0.1, 0.8, 0.2, 0.9].into_iter();
        assert_eq!(dense_reduce(4, 1, || rolls.next().unwrap()), 2);
        assert_eq!(dense_reduce(4, 0, || panic!("no random draw for zero level")), 4);
        assert_eq!(repair_factor(2, 0), 2.0);
        assert!((durability_boost(1, 1) - 0.25).abs() < 0.00001);
        assert_eq!(boosted_durability(125.0, 100.0, 1, 0), 140);
        assert_eq!(boosted_durability(125.0, 100.0, 0, 1), 135);
        assert_eq!(boosted_durability(125.0, 100.0, 1, 1), 150);
        assert!((repair_factor(1, 1) - 1.725).abs() < 0.00001);
        assert_eq!(wear_damage_bonus(3.0, 0, 1.0, 1, 0), 0.0);
        assert!((wear_damage_bonus(3.0, 100, 1.0, 1, 0) - 0.15).abs() < 0.00001);
        assert!((wear_damage_bonus(3.0, 100, 1.0, 0, 1) + 0.15).abs() < 0.00001);
        assert!((wear_damage_bonus(3.0, 100, 1.0, 1, 1) + 0.0075).abs() < 0.00001);
    }
}

// Existing upgrade compounds are key-sorted. Preserve new insertion history in
// a separate list; old items have no recoverable history and use sorted keys.
pub(super) fn reconcile_upgrade_order(custom: &mut horizon_world::nbt::Tag, added: Option<&str>) -> Option<()> {
    use horizon_world::nbt::Tag;
    let Tag::Compound(fields) = custom else { return None; };
    let Tag::Compound(upgrades) = fields.get(&"tic_upgrades".into())? else { return None; };
    let mut ids: Vec<String> = match fields.get(&"tic_upgrade_order".into()) {
        Some(Tag::List(list)) => list.iter().filter_map(|v| if let Tag::String(s) = v { Some(s.to_string_lossy().to_string()) } else { None }).collect(),
        _ => Vec::new(),
    };
    ids.retain(|id| upgrades.contains_key(&id.as_str().into()));
    let mut seen = std::collections::BTreeSet::new();
    ids.retain(|id| seen.insert(id.clone()));
    for id in upgrades.keys().map(|id| id.to_string_lossy().to_string()) {
        if Some(id.as_str()) != added && !ids.contains(&id) { ids.push(id); }
    }
    if let Some(id) = added {
        if upgrades.contains_key(&id.into()) && !ids.iter().any(|v| v == id) { ids.push(id.to_owned()); }
    }
    fields.insert("tic_upgrade_order".into(), Tag::List(ids.iter().map(|id| Tag::String(id.as_str().into())).collect()));
    Some(())
}

pub(super) fn ordered_modifier_ids(stack: &horizon_items::items::ItemStack,
    levels: &std::collections::BTreeMap<String,i32>) -> Vec<String> {
    use horizon_world::nbt::Tag;
    let custom = stack.data().and_then(|d| d.other("minecraft:custom_data"));
    let upgrades = custom.and_then(|d| d.get("tic_upgrades"));
    let list = |key| -> Vec<String> { match custom.and_then(|c| c.get(key)) {
        Some(Tag::List(values)) => values.iter().filter_map(|v| if let Tag::String(s)=v {Some(s.to_string_lossy().to_string())} else {None}).collect(),
        _ => Vec::new(),
    }};
    let mut ids = list("tic_upgrade_order");
    if let Some(Tag::Compound(entries)) = upgrades {
        for id in entries.keys().map(|v|v.to_string_lossy().to_string()) { if !ids.contains(&id) { ids.push(id); } }
    }
    for id in list("tic_trait_order") { if !ids.contains(&id) { ids.push(id); } }
    for id in levels.keys() { if !ids.contains(id) { ids.push(id.clone()); } }
    let mut seen = std::collections::BTreeSet::new();
    ids.retain(|id| seen.insert(id.clone()));
    ids
}

pub(super) fn ordered_durability_multiplier(stack: &horizon_items::items::ItemStack,
    stats: &super::tconstruct_source_station_stats::BaseStats,
    levels: &std::collections::BTreeMap<String,i32>, include_overlord: bool) -> f32 {
    use horizon_world::nbt::Tag;
    let upgrades = stack.data().and_then(|d| d.other("minecraft:custom_data")).and_then(|d| d.get("tic_upgrades"));
    let mut percent = stats.durability_multiplier_unit;
    for id in ordered_modifier_ids(stack, levels) {
        let trait_level = levels.get(&id).copied().unwrap_or(0).max(0);
        let upgrade = match upgrades.and_then(|u|u.get(id.as_str())) {Some(Tag::Int(n)) => (*n).max(0), _=>0};
        let delta = match id.as_str() {
            "tconstruct:preserved" => 0.15 * (trait_level + upgrade) as f32,
            "tconstruct:ductile" => 0.1 * (trait_level + upgrade) as f32,
            "tconstruct:emerald" if upgrade > 0 || trait_level > 0 => 0.5,
            "tconstruct:netherite" if upgrade > 0 || trait_level > 0 => 0.2,
            _ => 0.0,
        };
        percent *= 1.0 + delta;
    }
    if include_overlord {
        let level = levels.get("tconstruct:overlord").copied().unwrap_or(0).max(0);
        if level > 0 { percent *= 1.0 + if level >= 7 { -0.99999 } else { -0.15 * level as f32 }; }
    }
    percent
}

pub fn store_trait_order(stack: &mut horizon_items::items::ItemStack, order: &[String]) -> Option<()> {
    use horizon_world::nbt::Tag;
    let mut custom = stack.data()?.other("minecraft:custom_data")?.clone();
    let Tag::Compound(fields) = &mut custom else { return None; };
    fields.insert("tic_trait_order".into(), Tag::List(order.iter().map(|id| Tag::String(id.as_str().into())).collect()));
    stack.data_mut().set_other("minecraft:custom_data", Some(custom));
    Some(())
}

#[cfg(test)]
mod source_order_tests {
    use super::*;
    use horizon_items::items::ItemStack;
    use horizon_world::nbt::Tag;
    use std::collections::BTreeMap;

    #[test]
    fn trait_multiplier_order_preserves_float_boundary() {
        let def = br#"{"modules":[{"type":"tconstruct:base_stats","stats":{"tconstruct:durability":100}}]}"#;
        let stats = super::super::tconstruct_source_station_stats::base_stats(def, &[]).unwrap();
        let levels = BTreeMap::from([("tconstruct:ductile".into(),1), ("tconstruct:preserved".into(),4)]);
        let make = |first: &str, second: &str| {
            let mut stack = ItemStack::new("tconstruct:pickaxe", 1);
            stack.data_mut().set_other("minecraft:custom_data", Some(Tag::compound([
                ("tic_upgrades", Tag::compound([("tconstruct:netherite",Tag::Int(1)),("tconstruct:emerald",Tag::Int(1))])),
                ("tic_upgrade_order", Tag::List(vec![Tag::String("tconstruct:netherite".into()), Tag::String("tconstruct:emerald".into())])),
                ("tic_trait_order", Tag::List(vec![Tag::String(first.into()), Tag::String(second.into())])),
            ]))); stack
        };
        let first = ordered_durability_multiplier(&make("tconstruct:ductile", "tconstruct:preserved"), &stats, &levels, false);
        let second = ordered_durability_multiplier(&make("tconstruct:preserved", "tconstruct:ductile"), &stats, &levels, false);
        assert_eq!(first.to_bits(), (((1.0f32 * 1.2) * 1.5 * 1.1) * (1.0 + 0.15f32 * 4.0)).to_bits());
        assert_eq!(second.to_bits(), (((1.0f32 * 1.2) * 1.5 * (1.0 + 0.15f32 * 4.0)) * 1.1).to_bits());
        // Java rounds each multiplication in modifier order; equal results are valid.
    }

    #[test]
    fn overlord_copies_before_its_priority_50_reduction() {
        let def = br#"{"modules":[{"type":"tconstruct:base_stats","stats":{"tconstruct:durability":100}}]}"#;
        let stats = super::super::tconstruct_source_station_stats::base_stats(def, &[]).unwrap();
        let mut stack = ItemStack::new("tconstruct:pickaxe", 1);
        stack.data_mut().set_other("minecraft:custom_data", Some(Tag::compound([
            ("tic_upgrades", Tag::compound([("tconstruct:emerald",Tag::Int(1)),("tconstruct:netherite",Tag::Int(1))])),
            ("tic_upgrade_order", Tag::List(vec![Tag::String("tconstruct:emerald".into()),Tag::String("tconstruct:netherite".into())])),
            ("tic_trait_order", Tag::List(vec![Tag::String("tconstruct:ductile".into()),Tag::String("tconstruct:overlord".into())])),
        ])));
        let levels = BTreeMap::from([("tconstruct:ductile".into(),1),("tconstruct:overlord".into(),2)]);
        let before_copy = ordered_durability_multiplier(&stack, &stats, &levels, false);
        let after = ordered_durability_multiplier(&stack, &stats, &levels, true);
        assert!((before_copy - 1.5 * 1.2 * 1.1).abs() < 0.00001);
        assert!((after - 1.5 * 1.2 * 1.1 * 0.7).abs() < 0.00001);
    }

    #[test]
    fn new_upgrade_follows_existing_and_removal_discards_its_order() {
        let mut custom = Tag::compound([("tic_upgrades", Tag::compound([("z",Tag::Int(1)),("a",Tag::Int(1))]))]);
        reconcile_upgrade_order(&mut custom, Some("a")).unwrap();
        let Some(Tag::List(order)) = custom.get("tic_upgrade_order") else { panic!() };
        assert_eq!(order, &vec![Tag::String("z".into()), Tag::String("a".into())]);
        let Tag::Compound(fields) = &mut custom else { panic!() };
        let Some(Tag::Compound(upgrades)) = fields.get_mut(&"tic_upgrades".into()) else { panic!() };
        upgrades.remove(&"z".into());
        reconcile_upgrade_order(&mut custom, None).unwrap();
        let Some(Tag::List(order)) = custom.get("tic_upgrade_order") else { panic!() };
        assert_eq!(order, &vec![Tag::String("a".into())]);
    }
}
