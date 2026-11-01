use horizon_items::items::{ItemStack, Items};
use horizon_world::nbt::Tag;
use std::collections::{BTreeMap, HashSet};
#[path = "../../mixins/tconstruct/source-mining-math.rs"] mod tconstruct_source_mining_math;
#[path = "../../mixins/tconstruct/source-station-stats.rs"] mod tconstruct_source_station_stats;
#[path = "../../mixins/tconstruct/source-mining-stack.rs"] mod tconstruct_source_mining_stack;
#[path = "../../mixins/tconstruct/source-mining-rules.rs"] mod tconstruct_source_mining_rules;
include!("../../mixins/tconstruct/source-mining-return.rs");
include!("../../mixins/tconstruct/source-harvest-return.rs");

#[test]
fn per_stack_rules_respect_effective_blocks_broken_and_material_swap() {
    let definition = br#"{"modules":[{"type":"tconstruct:part_stats","parts":["tconstruct:pick_head","tconstruct:tool_handle"]}]}"#;
    let head = serde_json::json!({"tconstruct:head":{"durability":100,"mining_speed":3.5}}).as_object().unwrap().clone();
    let handle = serde_json::json!({"tconstruct:handle":{"durability":0,"mining_speed":0}}).as_object().unwrap().clone();
    let stats = tconstruct_source_station_stats::base_stats(definition, &[head, handle]).unwrap();
    let mut stack = ItemStack::new("tconstruct:pickaxe", 1);
    stack.data_mut().set_other("minecraft:custom_data", Some(Tag::compound([("tic_max_damage", Tag::Int(100))])));
    stack.data_mut().set_other("minecraft:tool", Some(Tag::compound([("rules", Tag::List(vec![
        Tag::compound([("blocks", Tag::String("#minecraft:mineable/pickaxe".into())), ("speed", Tag::Float(3.5f32.to_bits()))])
    ]))])));
    let mut items = Items::tags_only(std::path::Path::new("/tmp")).unwrap();
    items.block_tags.insert("minecraft:mineable/pickaxe".into(), HashSet::from(["minecraft:stone".into()]));
    let levels = BTreeMap::from([("tconstruct:lightweight".to_string(), 1), ("tconstruct:jagged".to_string(), 1)]);
    let order = vec!["tconstruct:lightweight".to_string(), "tconstruct:jagged".to_string()];
    tconstruct_source_mining_stack::apply(&mut stack, &stats, &levels, &order, true).unwrap();
    assert!((tconstruct_material_mining_speed(1.0, Some(&stack), "minecraft:stone", &items) - 3.78).abs() < 0.0001);
    // The vanilla wire codec may omit the tool patch; synchronized custom data retains the rules.
    stack.data_mut().set_other("minecraft:tool", None);
    assert!((tconstruct_material_mining_speed(1.0, Some(&stack), "minecraft:stone", &items) - 3.78).abs() < 0.0001);
    assert_eq!(tconstruct_material_mining_speed(1.0, Some(&stack), "minecraft:dirt", &items), 1.0);
    stack.damage = 25;
    let base = tconstruct_material_mining_speed(1.0, Some(&stack), "minecraft:stone", &items);
    assert!((base - 3.78).abs() < 0.0001);
    let custom = stack.data().unwrap().other("minecraft:custom_data").unwrap();
    // Efficiency is added before the source percentage modifier; Haste scales both.
    assert!((tconstruct_source_mining_rules::conditional(custom, 25, (base + 1.0) * 1.2, 1.2).unwrap() - 5.4492).abs() < 0.0001);
    stack.damage = 100;
    assert_eq!(tconstruct_material_mining_speed(0.3, Some(&stack), "minecraft:stone", &items), 0.3);
    stack.damage = 25;
    let cached = stack.data().unwrap().other("minecraft:custom_data").unwrap().get("tic_mining_tool").unwrap().clone();
    stack.data_mut().set_other("minecraft:tool", Some(cached));
    tconstruct_source_mining_stack::apply(&mut stack, &stats, &BTreeMap::new(), &[], true).unwrap();
    assert!((tconstruct_material_mining_speed(1.0, Some(&stack), "minecraft:stone", &items) - 3.5).abs() < 0.0001);
}

#[test]
fn unsupported_harvest_tool_without_rules_still_builds() {
    let definition = br#"{"modules":[{"type":"tconstruct:base_stats","stats":{"tconstruct:mining_speed":2.0}}]}"#;
    let stats = tconstruct_source_station_stats::base_stats(definition, &[]).unwrap();
    let mut stack = ItemStack::new("tconstruct:flint_and_brick", 1);
    assert!(tconstruct_source_mining_stack::apply(&mut stack, &stats, &BTreeMap::new(), &[], true).is_some());
}

#[test]
fn material_tier_effectiveness_and_broken_state_control_real_harvest() {
    let mut items = Items::tags_only(std::path::Path::new("/tmp")).unwrap();
    items.block_tags.insert("minecraft:incorrect_for_stone_tool".into(), HashSet::from(["minecraft:diamond_ore".into()]));
    items.block_tags.insert("tconstruct:source_effective/minecraft/mineable/pickaxe".into(), HashSet::from(["minecraft:stone".into(), "minecraft:diamond_ore".into()]));
    let rules = |tier: &str| Tag::compound([("rules", Tag::List(vec![
        Tag::compound([("blocks", Tag::String(format!("#minecraft:incorrect_for_{tier}_tool").as_str().into())), ("correct_for_drops", Tag::Byte(0))]),
        Tag::compound([("blocks", Tag::String("#tconstruct:source_effective/minecraft/mineable/pickaxe".into())), ("speed", Tag::Float(7.0f32.to_bits())), ("correct_for_drops", Tag::Byte(1))]),
    ]))]);
    let mut pick = ItemStack::new("tconstruct:pickaxe", 1);
    let data = |tier: &str| Tag::compound([("tic_max_damage", Tag::Int(100)), ("tic_mining_traits", Tag::List(vec![])), ("tic_mining_tool", rules(tier))]);
    pick.data_mut().set_other("minecraft:custom_data", Some(data("stone")));
    assert!(tconstruct_material_correct_for_drops(false, Some(&pick), "minecraft:stone", &items));
    assert!(!tconstruct_material_correct_for_drops(true, Some(&pick), "minecraft:diamond_ore", &items));
    assert_eq!(tconstruct_material_mining_speed(3.5, Some(&pick), "minecraft:diamond_ore", &items), 1.0);
    pick.data_mut().set_other("minecraft:custom_data", Some(data("diamond")));
    assert!(tconstruct_material_correct_for_drops(false, Some(&pick), "minecraft:diamond_ore", &items));
    assert_eq!(tconstruct_material_mining_speed(3.5, Some(&pick), "minecraft:diamond_ore", &items), 7.0);
    assert!(!tconstruct_material_correct_for_drops(true, Some(&pick), "minecraft:dirt", &items));
    pick.damage = 100;
    assert!(!tconstruct_material_correct_for_drops(false, Some(&pick), "minecraft:stone", &items));
    assert_eq!(tconstruct_material_mining_speed(0.3, Some(&pick), "minecraft:stone", &items), 0.3);
    let vanilla = ItemStack::new("minecraft:iron_pickaxe", 1);
    assert!(tconstruct_material_correct_for_drops(true, Some(&vanilla), "minecraft:stone", &items));
}
