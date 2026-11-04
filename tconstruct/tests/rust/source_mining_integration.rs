use horizon_items::items::{ItemStack, Items};
#[path = "../../../pluto-lib/mixins/pluto_tool_rules/rules.rs"] pub mod rules;
mod tconstruct_tool_api {
    pub use horizon_items::item_data::canon_tag;
    pub use crate::rules;
}
#[path = "../../mixins/tconstruct/source-dwarven.rs"] mod tconstruct_source_dwarven;
use horizon_world::nbt::Tag;
use std::collections::{BTreeMap, HashSet};
#[path = "../../mixins/tconstruct/source-mining-math.rs"] mod tconstruct_source_mining_math;
#[path = "../../mixins/tconstruct/source-station-stats.rs"] mod tconstruct_source_station_stats;
#[path = "../../mixins/tconstruct/source-material-effects.rs"] mod tconstruct_source_material_effects;
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
    let mut items = Items::tags_only(std::path::Path::new("/tmp"), &horizon_world::loot::PackOverlay::NONE).unwrap();
    items.block_tags.insert("minecraft:mineable/pickaxe".into(), HashSet::from(["minecraft:stone".into()]));
    let levels = BTreeMap::from([("tconstruct:lightweight".to_string(), 1), ("tconstruct:jagged".to_string(), 1)]);
    let order = vec!["tconstruct:lightweight".to_string(), "tconstruct:jagged".to_string()];
    tconstruct_source_mining_stack::apply(&mut stack, &stats, &levels, &order, true).unwrap();
    assert!((tconstruct_material_mining_speed(1.0, Some(&stack), "minecraft:stone", &items) - 3.78).abs() < 0.0001);
    // Absent Tool falls back to the host result; restore the actual component.
    let tool = stack.data().unwrap().other("minecraft:tool").unwrap().clone();
    stack.data_mut().set_other("minecraft:tool", None);
    assert_eq!(tconstruct_material_mining_speed(1.0, Some(&stack), "minecraft:stone", &items), 1.0);
    stack.data_mut().set_other("minecraft:tool", Some(tool));
    assert_eq!(tconstruct_material_mining_speed(1.0, Some(&stack), "minecraft:dirt", &items), 1.0);
    stack.damage = 25;
    let base = tconstruct_material_mining_speed(1.0, Some(&stack), "minecraft:stone", &items);
    assert!((base - 3.78).abs() < 0.0001);
    let custom = stack.data().unwrap().other("minecraft:custom_data").unwrap();
    // Efficiency is added before the source percentage modifier; Haste scales both.
    assert!((tconstruct_source_mining_rules::conditional(custom, 25, (base + 1.0) * 1.2, 1.2, None, true, false, Some(false)).unwrap() - 5.4492).abs() < 0.0001);
    stack.damage = 100;
    assert_eq!(tconstruct_material_mining_speed(0.3, Some(&stack), "minecraft:stone", &items), 1.0);
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
    let mut items = Items::tags_only(std::path::Path::new("/tmp"), &horizon_world::loot::PackOverlay::NONE).unwrap();
    items.block_tags.insert("minecraft:incorrect_for_stone_tool".into(), HashSet::from(["minecraft:diamond_ore".into()]));
    items.block_tags.insert("tconstruct:source_effective/minecraft/mineable/pickaxe".into(), HashSet::from(["minecraft:stone".into(), "minecraft:diamond_ore".into()]));
    let rules = |tier: &str| Tag::compound([("rules", Tag::List(vec![
        Tag::compound([("blocks", Tag::String(format!("#minecraft:incorrect_for_{tier}_tool").as_str().into())), ("correct_for_drops", Tag::Byte(0))]),
        Tag::compound([("blocks", Tag::String("#tconstruct:source_effective/minecraft/mineable/pickaxe".into())), ("speed", Tag::Float(7.0f32.to_bits())), ("correct_for_drops", Tag::Byte(1))]),
    ]))]);
    let mut pick = ItemStack::new("tconstruct:pickaxe", 1);
    let data = |tier: &str| Tag::compound([("tic_max_damage", Tag::Int(100)), ("tic_mining_traits", Tag::List(vec![])), ("tic_mining_tool", rules(tier))]);
    pick.data_mut().set_other("minecraft:custom_data", Some(data("stone")));
    pick.data_mut().set_other("minecraft:tool", Some(rules("stone")));
    assert!(tconstruct_material_correct_for_drops(false, Some(&pick), "minecraft:stone", &items));
    assert!(!tconstruct_material_correct_for_drops(true, Some(&pick), "minecraft:diamond_ore", &items));
    assert_eq!(tconstruct_material_mining_speed(3.5, Some(&pick), "minecraft:diamond_ore", &items), 1.0);
    pick.data_mut().set_other("minecraft:custom_data", Some(data("diamond")));
    pick.data_mut().set_other("minecraft:tool", Some(rules("diamond")));
    assert!(tconstruct_material_correct_for_drops(false, Some(&pick), "minecraft:diamond_ore", &items));
    assert_eq!(tconstruct_material_mining_speed(3.5, Some(&pick), "minecraft:diamond_ore", &items), 7.0);
    assert!(!tconstruct_material_correct_for_drops(true, Some(&pick), "minecraft:dirt", &items));
    pick.damage = 100;
    assert!(!tconstruct_material_correct_for_drops(false, Some(&pick), "minecraft:stone", &items));
    assert_eq!(tconstruct_material_mining_speed(0.3, Some(&pick), "minecraft:stone", &items), 1.0);
    let vanilla = ItemStack::new("minecraft:iron_pickaxe", 1);
    assert!(tconstruct_material_correct_for_drops(true, Some(&vanilla), "minecraft:stone", &items));
}

#[test]
fn harvest_tier_uses_only_materials_applied_as_head_stats() {
    use serde_json::{json, Map, Value};
    let mat = |tier: &str| -> Map<String, Value> { json!({"tconstruct:head":{"mining_tier":tier}}).as_object().unwrap().clone() };
    let parts = br#"{"modules":[{"type":"tconstruct:part_stats","parts":["tconstruct:pick_head","tconstruct:tool_handle","tconstruct:tool_binding"]}]}"#;
    let flint = mat("minecraft:stone"); let cobalt = mat("minecraft:diamond");
    assert_eq!(tconstruct_source_station_stats::mining_tier(parts, &[flint.clone(), cobalt.clone(), cobalt.clone()]), Some(Some("stone")));
    assert_eq!(tconstruct_source_station_stats::mining_tier(parts, &[cobalt.clone(), flint.clone(), flint.clone()]), Some(Some("diamond")));
    let multi = br#"{"modules":[{"type":"tconstruct:material_stats","stat_types":["tconstruct:head","tconstruct:head","tconstruct:handle"]}]}"#;
    assert_eq!(tconstruct_source_station_stats::mining_tier(multi, &[flint.clone(), cobalt.clone(), flint.clone()]), Some(Some("diamond")));
    assert_eq!(tconstruct_source_station_stats::mining_tier(multi, &[flint.clone(), flint, cobalt]), Some(Some("stone")));
}

#[test]
fn gem_mining_applies_additive_speed_and_conditional_multiplier_separately() {
    let def = br#"{"modules":[{"type":"tconstruct:base_stats","stats":{"tconstruct:mining_speed":4.0}}]}"#;
    let stats = tconstruct_source_station_stats::base_stats(def, &[]).unwrap();
    let mut stack = ItemStack::new("tconstruct:pickaxe",1);
    stack.data_mut().set_other("minecraft:custom_data", Some(Tag::compound([
        ("tic_max_damage",Tag::Int(100)),
        ("tic_upgrades",Tag::compound([("tconstruct:diamond",Tag::Int(1)),("tconstruct:emerald",Tag::Int(1))]))
    ])));
    stack.data_mut().set_other("minecraft:tool",Some(Tag::compound([("rules",Tag::List(vec![Tag::compound([("speed",Tag::Float(4.0f32.to_bits()))])]))])));
    tconstruct_source_mining_stack::apply(&mut stack,&stats,&BTreeMap::from([("tconstruct:maintained".into(),1)]),&["tconstruct:maintained".into()],true).unwrap();
    let custom=stack.data().unwrap().other("minecraft:custom_data").unwrap();
    let Some(Tag::List(rules))=custom.get("tic_mining_tool").and_then(|t|t.get("rules")) else {panic!()};
    assert!(matches!(rules[0].get("speed"),Some(Tag::Float(v)) if (f32::from_bits(*v)-6.0).abs()<0.0001));
    assert!((tconstruct_source_mining_rules::conditional(custom,25,6.0,1.0,None,true,false,Some(false)).unwrap()-9.75).abs()<0.0001);
}

#[test]
fn netherite_and_haste_rebuild_mining_speed_without_stacking() {
    let definition = br#"{"modules":[{"type":"tconstruct:base_stats","stats":{"tconstruct:mining_speed":4.0}}]}"#;
    let stats = tconstruct_source_station_stats::base_stats(definition, &[]).unwrap();
    let mut tool = ItemStack::new("tconstruct:pickaxe", 1);
    tool.data_mut().set_other("minecraft:custom_data", Some(Tag::compound([("tic_upgrades", Tag::compound([("tconstruct:netherite", Tag::Int(1)), ("tconstruct:haste", Tag::Int(45))]))])));
    tool.data_mut().set_other("minecraft:tool", Some(Tag::compound([("rules", Tag::List(vec![Tag::compound([("speed", Tag::Float(4.0f32.to_bits()))])]))])));
    for _ in 0..2 {
        tconstruct_source_mining_stack::apply(&mut tool, &stats, &BTreeMap::new(), &[], true).unwrap();
        let custom = tool.data().unwrap().other("minecraft:custom_data").unwrap();
        let Some(Tag::List(rules)) = custom.get("tic_mining_tool").and_then(|tool| tool.get("rules")) else { panic!() };
        let Some(Tag::Float(speed)) = rules[0].get("speed") else { panic!() };
        assert!((f32::from_bits(*speed) - 10.0).abs() < 0.0001);
    }
}

#[test]
fn conditional_mining_multiplier_respects_emerald_trait_and_modifier_order() {
 let definition=br#"{"modules":[{"type":"tconstruct:base_stats","stats":{"tconstruct:mining_speed":4.0}},{"type":"tconstruct:multiply_stats","multipliers":{"tconstruct:mining_speed":0.3}}]}"#;
 let stats=tconstruct_source_station_stats::base_stats(definition,&[]).unwrap();
 let run=|order:&[&str], upgraded:bool| {
  let mut stack=ItemStack::new("tconstruct:pickaxe",1);
  let upgrades=if upgraded {Tag::compound([("tconstruct:emerald",Tag::Int(1))])} else {Tag::compound(Vec::<(&str,Tag)>::new())};
  stack.data_mut().set_other("minecraft:custom_data",Some(Tag::compound([
   ("tic_max_damage",Tag::Int(100)),("tic_upgrades",upgrades),
   ("tic_trait_order",Tag::List(order.iter().map(|id|Tag::String((*id).into())).collect()))
  ])));
  let levels=BTreeMap::from([("tconstruct:vintage".into(),1),("tconstruct:emerald".into(),1)]);
  tconstruct_source_mining_stack::apply(&mut stack,&stats,&levels,&[],true).unwrap();
  let custom=stack.data().unwrap().other("minecraft:custom_data").unwrap();
  match custom.get("tic_mining_all") {Some(Tag::Float(bits))=>f32::from_bits(*bits),_=>panic!("missing multiplier")}
 };
 let first=(0.3f32*1.25)*0.9; let second=(0.3f32*0.9)*1.25;
 assert_ne!(first.to_bits(),second.to_bits());
 assert_eq!(run(&["tconstruct:emerald","tconstruct:vintage"],false).to_bits(),first.to_bits());
 assert_eq!(run(&["tconstruct:vintage","tconstruct:emerald"],false).to_bits(),second.to_bits());
 // An upgrade precedes traits and combines with Emerald trait only once.
 assert_eq!(run(&["tconstruct:vintage"],true).to_bits(),first.to_bits());
}

#[test]
fn emerald_trait_scales_maintained_and_rebuild_removes_bonus() {
 let def=br#"{"modules":[{"type":"tconstruct:base_stats","stats":{"tconstruct:mining_speed":4.0}}]}"#;
 let stats=tconstruct_source_station_stats::base_stats(def,&[]).unwrap();
 let mut stack=ItemStack::new("tconstruct:pickaxe",1);
 stack.data_mut().set_other("minecraft:custom_data",Some(Tag::compound([("tic_max_damage",Tag::Int(100))])));
 let order=vec!["tconstruct:maintained".to_owned()];
 let with_emerald=BTreeMap::from([("tconstruct:emerald".into(),1),("tconstruct:maintained".into(),1)]);
 tconstruct_source_mining_stack::apply(&mut stack,&stats,&with_emerald,&order,true).unwrap();
 tconstruct_source_mining_stack::apply(&mut stack,&stats,&with_emerald,&order,true).unwrap();
 let custom=stack.data().unwrap().other("minecraft:custom_data").unwrap();
 assert!((tconstruct_source_mining_rules::conditional(custom,0,4.0,1.0,None,true,false,Some(false)).unwrap()-11.5).abs()<1e-6);
 let without_emerald=BTreeMap::from([("tconstruct:maintained".into(),1)]);
 tconstruct_source_mining_stack::apply(&mut stack,&stats,&without_emerald,&order,true).unwrap();
 let custom=stack.data().unwrap().other("minecraft:custom_data").unwrap();
 assert!((tconstruct_source_mining_rules::conditional(custom,0,4.0,1.0,None,true,false,Some(false)).unwrap()-10.0).abs()<1e-6);
}

#[test]
fn diamond_trait_mining_speed_is_flat_and_rebuilt_once() {
 let def=br#"{"modules":[{"type":"tconstruct:base_stats","stats":{"tconstruct:mining_speed":4.0}},{"type":"tconstruct:multiply_stats","multipliers":{"tconstruct:mining_speed":2.0}}]}"#;
 let stats=tconstruct_source_station_stats::base_stats(def,&[]).unwrap();
 let mut tool=ItemStack::new("tconstruct:pickaxe",1);
 tool.data_mut().set_other("minecraft:custom_data",Some(Tag::compound(Vec::<(&str,Tag)>::new())));
 tool.data_mut().set_other("minecraft:tool",Some(Tag::compound([("rules",Tag::List(vec![Tag::compound([("speed",Tag::Float(1.0f32.to_bits()))])]))])));
 let traits=BTreeMap::from([("tconstruct:diamond".into(),2)]);
 let speed=|tool:&ItemStack| {let t=tool.data().unwrap().other("minecraft:tool").unwrap(); let Some(Tag::List(rules))=t.get("rules") else {panic!()}; match rules[0].get("speed"){Some(Tag::Float(v))=>f32::from_bits(*v),_=>panic!()}};
 for _ in 0..2 {tconstruct_source_mining_stack::apply(&mut tool,&stats,&traits,&[],true).unwrap(); assert_eq!(speed(&tool),12.0);}
 let mut custom=tool.data().unwrap().other("minecraft:custom_data").unwrap().clone(); let Tag::Compound(ref mut fields)=custom else {panic!()};
 fields.insert("tic_upgrades".into(),Tag::compound([("tconstruct:diamond",Tag::Int(1))])); tool.data_mut().set_other("minecraft:custom_data",Some(custom));
 tconstruct_source_mining_stack::apply(&mut tool,&stats,&traits,&[],true).unwrap(); assert_eq!(speed(&tool),12.0);
 let mut custom=tool.data().unwrap().other("minecraft:custom_data").unwrap().clone(); let Tag::Compound(ref mut fields)=custom else {panic!()}; fields.remove(&"tic_upgrades".into()); tool.data_mut().set_other("minecraft:custom_data",Some(custom));
 tconstruct_source_mining_stack::apply(&mut tool,&stats,&BTreeMap::new(),&[],true).unwrap(); assert_eq!(speed(&tool),8.0);
}

#[test]
fn diamond_trait_does_not_change_nonharvest_tool() {
 let stats=tconstruct_source_station_stats::base_stats(br#"{"modules":[]}"#,&[]).unwrap();
 let mut tool=ItemStack::new("tconstruct:pickaxe",1);
 let custom=Tag::compound([("fixture",Tag::Int(7))]);
 tool.data_mut().set_other("minecraft:custom_data",Some(custom.clone()));
 tconstruct_source_mining_stack::apply(&mut tool,&stats,&BTreeMap::from([("tconstruct:diamond".into(),1)]),&[],false).unwrap();
 assert_eq!(tool.data().unwrap().other("minecraft:custom_data"),Some(&custom));
}
