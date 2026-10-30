// Supported source trait formulas. Other modifier effects are not interpreted here.
pub(super) fn repair_factor(cultivated: i32, preserved: i32) -> f32 {
    (1.0 + cultivated as f32 * 0.5) * (1.0 + preserved as f32 * 0.15)
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
