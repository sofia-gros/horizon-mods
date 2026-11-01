// Supported source trait formulas. Other modifier effects are not interpreted here.
pub(super) fn repair_factor(cultivated: i32, preserved: i32) -> f32 {
    (1.0 + cultivated as f32 * 0.5) * (1.0 + preserved as f32 * 0.15)
}

/// Dense repair follows the two level ranges in the original repair module.
pub(super) fn dense_repair_factor(level: i32) -> f32 {
    match level {
        i32::MIN..=0 => 1.0,
        1..=5 => 1.0 - 0.025 * level as f32 * (11 - level) as f32,
        _ => (0.5 - 0.05 * level as f32).max(0.0),
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
pub(super) fn apply_stat_traits(
    stack: &mut horizon_items::items::ItemStack,
    stats: &super::tconstruct_source_station_stats::BaseStats,
    levels: &std::collections::BTreeMap<String, i32>,
) -> Option<()> {
    let preserved = levels.get("tconstruct:preserved").copied().unwrap_or(0);
    let ductile = levels.get("tconstruct:ductile").copied().unwrap_or(0);
    let boost = durability_boost(preserved, ductile);
    if boost == 0.0 {
        return Some(());
    }
    let durability = boosted_durability(
        stats.durability,
        stats.durability_boost_unit,
        preserved,
        ductile,
    );
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
        let toughness = stats.armor_toughness + ductile as f32;
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
