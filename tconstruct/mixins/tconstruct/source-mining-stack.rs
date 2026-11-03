// Store source mining stats and ordered dynamic traits during regeneration.
use horizon_items::items::ItemStack;
use horizon_world::nbt::Tag;
use std::collections::BTreeMap;

pub fn apply(stack: &mut ItemStack, stats: &super::tconstruct_source_station_stats::BaseStats,
    levels: &BTreeMap<String, i32>, order: &[String], harvest: bool) -> Option<()> {
    if !harvest { return Some(()); }
    let level = |id: &str| levels.get(id).copied().unwrap_or(0);
    let vintage = level("tconstruct:vintage");
    let haste = match stack.data().and_then(|data| data.other("minecraft:custom_data"))
        .and_then(|custom| custom.get("tic_upgrades"))
        .and_then(|upgrades| upgrades.get("tconstruct:haste")) {
        Some(Tag::Int(amount)) => (*amount).max(0) as f32 / 45.0,
        _ => 0.0,
    };
    let has_gem = |id| matches!(stack.data().and_then(|d| d.other("minecraft:custom_data")).and_then(|d| d.get("tic_upgrades")).and_then(|d| d.get(id)), Some(Tag::Int(v)) if *v > 0);
    let diamond = has_gem("tconstruct:diamond");
    let emerald = has_gem("tconstruct:emerald");
    let netherite = has_gem("tconstruct:netherite") || level("tconstruct:netherite") > 0;
    let mut add = stats.mining_speed_add_raw;
    let percent = stats.mining_speed_percent_raw;
    let mut multiplier = stats.mining_speed_all_multiplier;
    for id in super::tconstruct_source_material_effects::ordered_modifier_ids(stack, levels) {
        match id.as_str() {
            "tconstruct:diamond" if diamond => add += 2.0,
            "tconstruct:haste" => add += 4.0 * (haste + level(&id).max(0) as f32),
            "tconstruct:entangled" if level(&id) > 0 => add += 6.0,
            "tconstruct:lightweight" => multiplier *= 1.0 + 0.08 * level(&id).max(0) as f32,
            "tconstruct:heavy" => multiplier *= 1.0 + 0.15 * level(&id).max(0) as f32,
            "tconstruct:netherite" if netherite => multiplier *= 1.25,
            "tconstruct:vintage" => multiplier *= 1.0 - 0.1 * vintage.max(0) as f32,
            _ => {},
        }
    }
    let speed = ((stats.mining_speed_base_raw + add) * (1.0 + percent) * multiplier).clamp(0.1, 2048.0);
    // A harvest-tagged tool without an explicit Tool component is still able
    // to carry traits such as Crumbling, which also applies when ineffective.
    let mut tool = stack.data().and_then(|data| data.other("minecraft:tool")).cloned();
    if let Some(ref mut tool) = tool {
        let Tag::Compound(fields) = tool else { return None; };
        let Some(Tag::List(rules)) = fields.get_mut(&"rules".into()) else { return None; };
        for rule in rules {
            let Tag::Compound(fields) = rule else { continue; };
            if let Some(Tag::Float(value)) = fields.get_mut(&"speed".into()) { *value = speed.to_bits(); }
        }
    }
    let Some(mut custom) = stack.data().and_then(|data| data.other("minecraft:custom_data")).cloned() else {
        return if tool.is_none() { Some(()) } else { None };
    };
    let Tag::Compound(ref mut fields) = custom else { return if tool.is_none() { Some(()) } else { None } };
    let dynamic: Vec<Tag> = order.iter().filter_map(|id| {
        if !matches!(id.as_str(), "tconstruct:jagged" | "tconstruct:stonebound" | "tconstruct:maintained" | "tconstruct:dwarven" | "tconstruct:crumbling" | "tconstruct:airborne" | "tconstruct:overwield") { return None; }
        Some(Tag::compound([("id", Tag::String(id.as_str().into())), ("level", Tag::Int(level(id)))]))
    }).collect();
    // Retain the existing generated-rule mirror for legacy consumers; the
    // shared transport also carries the supported Tool component.
    if let Some(ref tool) = tool { fields.insert("tic_mining_tool".into(), tool.clone()); }
    else { fields.remove(&"tic_mining_tool".into()); }
    fields.insert("tic_mining_traits".into(), Tag::List(dynamic));
    fields.insert("tic_mining_all".into(), Tag::Float((stats.mining_speed_all_multiplier * (1.0 - 0.1 * vintage.max(0) as f32) * if emerald { 1.25 } else { 1.0 }).to_bits()));
    fields.insert("tic_durability_all".into(), Tag::Float(stats.durability_multiplier_unit.to_bits()));
    if let Some(tool) = tool { stack.data_mut().set_other("minecraft:tool", Some(tool)); }
    stack.data_mut().set_other("minecraft:custom_data", Some(custom));
    Some(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn mining_base_multipliers_compound_before_application() {
        let definition = br#"{"modules":[{"type":"tconstruct:base_stats","stats":{"tconstruct:mining_speed":4.0}}]}"#;
        let stats = super::super::tconstruct_source_station_stats::base_stats(definition, &[]).unwrap();
        let mut stack = ItemStack::new("tconstruct:pickaxe", 1);
        stack.data_mut().set_other("minecraft:custom_data", Some(Tag::compound([("tic_trait_order",Tag::List(vec![Tag::String("tconstruct:lightweight".into()),Tag::String("tconstruct:heavy".into())]))])));
        stack.data_mut().set_other("minecraft:tool", Some(Tag::compound([("rules",Tag::List(vec![Tag::compound([("speed",Tag::Float(4.0f32.to_bits()))])]))])));
        let levels = BTreeMap::from([("tconstruct:lightweight".into(),1),("tconstruct:heavy".into(),1)]);
        apply(&mut stack, &stats, &levels, &[], true).unwrap();
        let tool = stack.data().unwrap().other("minecraft:tool").unwrap();
        let Some(Tag::List(rules)) = tool.get("rules") else { panic!() };
        assert!(matches!(rules[0].get("speed"), Some(Tag::Float(v)) if (f32::from_bits(*v)-4.968).abs()<0.0001));
    }

    #[test]
    fn mining_builder_keeps_add_percent_multipliers_and_harvest_scope() {
        let definition = br#"{"modules":[{"type":"tconstruct:base_stats","stats":{"tconstruct:mining_speed":4.0}}]}"#;
        let mut stats = super::super::tconstruct_source_station_stats::base_stats(definition, &[]).unwrap();
        stats.mining_speed_percent_raw = 0.2;
        let mut stack = ItemStack::new("tconstruct:pickaxe",1);
        stack.data_mut().set_other("minecraft:custom_data",Some(Tag::compound([
            ("tic_upgrades",Tag::compound([("tconstruct:diamond",Tag::Int(1)),("tconstruct:haste",Tag::Int(45)),("tconstruct:netherite",Tag::Int(1))])),
            ("tic_upgrade_order",Tag::List(vec![Tag::String("tconstruct:diamond".into()),Tag::String("tconstruct:haste".into()),Tag::String("tconstruct:netherite".into())])),
            ("tic_trait_order",Tag::List(vec![Tag::String("tconstruct:lightweight".into()),Tag::String("tconstruct:heavy".into()),Tag::String("tconstruct:vintage".into())])),
        ])));
        stack.data_mut().set_other("minecraft:tool",Some(Tag::compound([("rules",Tag::List(vec![Tag::compound([("speed",Tag::Float(4.0f32.to_bits()))])]))])));
        let levels=BTreeMap::from([("tconstruct:lightweight".into(),1),("tconstruct:heavy".into(),1),("tconstruct:vintage".into(),1)]);
        let before=stack.data().unwrap().other("minecraft:tool").unwrap().clone();
        apply(&mut stack,&stats,&levels,&[],false).unwrap();
        assert_eq!(stack.data().unwrap().other("minecraft:tool"),Some(&before));
        apply(&mut stack,&stats,&levels,&[],true).unwrap();
        let tool=stack.data().unwrap().other("minecraft:tool").unwrap();
        let Some(Tag::List(rules))=tool.get("rules") else {panic!()};
        // (4 + diamond 2 + Haste 4) * 1.2 * 1.25 * 1.08 * 1.15 * 0.9.
        assert!(matches!(rules[0].get("speed"),Some(Tag::Float(v)) if (f32::from_bits(*v)-16.767).abs()<0.0001));
    }

    #[test]
    fn crumbling_is_stored_without_tool_rules_and_removed_on_rebuild() {
        let definition = br#"{"modules":[{"type":"tconstruct:base_stats","stats":{"tconstruct:mining_speed":4.0}}]}"#;
        let stats = super::super::tconstruct_source_station_stats::base_stats(definition, &[]).unwrap();
        let mut stack = ItemStack::new("tconstruct:pickaxe", 1);
        stack.data_mut().set_other("minecraft:custom_data", Some(Tag::compound([("tic_upgrades", Tag::compound([("test:empty", Tag::Int(0))]))])));
        let id = "tconstruct:crumbling".to_owned();
        apply(&mut stack, &stats, &BTreeMap::from([(id.clone(), 1)]), &[id], true).unwrap();
        let data = stack.data().unwrap();
        assert!(data.other("minecraft:tool").is_none());
        let Some(Tag::List(traits)) = data.other("minecraft:custom_data").unwrap().get("tic_mining_traits") else { panic!() };
        assert_eq!(traits.len(), 1);
        apply(&mut stack, &stats, &BTreeMap::new(), &[], true).unwrap();
        let Some(Tag::List(traits)) = stack.data().unwrap().other("minecraft:custom_data").unwrap().get("tic_mining_traits") else { panic!() };
        assert!(traits.is_empty());
    }

    #[test]
    fn haste_partial_level_rebuilds_mining_speed_once() {
        let definition = br#"{"modules":[{"type":"tconstruct:base_stats","stats":{"tconstruct:mining_speed":4.0}}]}"#;
        let stats = super::super::tconstruct_source_station_stats::base_stats(definition, &[]).unwrap();
        let mut stack = ItemStack::new("tconstruct:pickaxe", 1);
        stack.data_mut().set_other("minecraft:custom_data", Some(Tag::compound([("tic_upgrades", Tag::compound([("tconstruct:haste", Tag::Int(18))]))])));
        stack.data_mut().set_other("minecraft:tool", Some(Tag::compound([("rules", Tag::List(vec![Tag::compound([("speed", Tag::Float(4.0f32.to_bits()))])]))])));
        for _ in 0..2 {
            apply(&mut stack, &stats, &BTreeMap::new(), &[], true).unwrap();
            let custom = stack.data().unwrap().other("minecraft:custom_data").unwrap();
            let Some(Tag::List(rules)) = custom.get("tic_mining_tool").and_then(|tool| tool.get("rules")) else { panic!() };
            let Some(Tag::Float(speed)) = rules[0].get("speed") else { panic!() };
            assert!((f32::from_bits(*speed) - 5.6).abs() < 0.0001);
        }
    }
    #[test]
    fn haste_trait_and_incremental_upgrade_rebuild_together() {
        let def=br#"{"modules":[{"type":"tconstruct:base_stats","stats":{"tconstruct:mining_speed":3}}]}"#;
        let stats=super::super::tconstruct_source_station_stats::base_stats(def,&[]).unwrap();
        let mut tool=ItemStack::new("tconstruct:pickaxe",1);
        tool.data_mut().set_other("minecraft:custom_data",Some(Tag::compound([("tic_upgrades",Tag::compound([("tconstruct:haste",Tag::Int(18))]))])));
        tool.data_mut().set_other("minecraft:tool",Some(Tag::compound([("rules",Tag::List(vec![Tag::compound([("speed",Tag::Float(3.0f32.to_bits()))])]))])));
        let traits=BTreeMap::from([("tconstruct:haste".into(),2)]);
        for (levels, expected) in [(&traits,12.6),(&BTreeMap::new(),4.6),(&traits,12.6)] {
            apply(&mut tool,&stats,levels,&[],true).unwrap();
            let Tag::List(rules)=tool.data().unwrap().other("minecraft:tool").unwrap().get("rules").unwrap() else { panic!("rules") };
            assert!(matches!(rules[0].get("speed"),Some(Tag::Float(v)) if (f32::from_bits(*v)-expected).abs()<1e-5));
        }
        tool.data_mut().set_other("minecraft:custom_data",Some(Tag::compound(Vec::<(&str,Tag)>::new())));
        apply(&mut tool,&stats,&traits,&[],true).unwrap();
        let Tag::List(rules)=tool.data().unwrap().other("minecraft:tool").unwrap().get("rules").unwrap() else { panic!("rules") };
        assert!(matches!(rules[0].get("speed"),Some(Tag::Float(v)) if (f32::from_bits(*v)-11.0).abs()<1e-5));
    }

}
