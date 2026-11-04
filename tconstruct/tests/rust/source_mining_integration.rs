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

#[test]
fn haste_mining_merges_logical_levels_before_float_scaling() {
    let definition = br#"{"modules":[{"type":"tconstruct:base_stats","stats":{"tconstruct:mining_speed":0.1}}]}"#;
    let stats = tconstruct_source_station_stats::base_stats(definition, &[]).unwrap();
    let mut items = Items::tags_only(std::path::Path::new("/tmp"), &horizon_world::loot::PackOverlay::NONE).unwrap();
    items.block_tags.insert("minecraft:mineable/pickaxe".into(), HashSet::from(["minecraft:stone".into()]));
    for (trait_level, amount) in [(0,1),(1,3),(2,6),(0,0),(2,0),(2,44),(2,45),(2,46),(2,89),(2,90),(2,91),(0,-1),(-1,1),(0,i32::MAX),(i32::MAX,1),(i32::MAX,45)] {
        let mut stack = ItemStack::new("tconstruct:pickaxe",1);
        let custom = Tag::compound([
            ("tic_upgrades",Tag::compound([("tconstruct:haste",Tag::Int(amount))])),
            ("fixture:foreign",Tag::String("preserve".into())),
            ("tic_max_damage",Tag::Int(100)),
        ]);
        let tool = Tag::compound([("rules",Tag::List(vec![Tag::compound([
            ("blocks",Tag::String("#minecraft:mineable/pickaxe".into())),
            ("speed",Tag::Float(0.1f32.to_bits())),
        ])]))]);
        stack.data_mut().set_other("minecraft:custom_data",Some(custom.clone()));
        stack.data_mut().set_other("minecraft:tool",Some(tool.clone()));
        let traits = BTreeMap::from([("tconstruct:haste".to_owned(),trait_level)]);
        tconstruct_source_mining_stack::apply(&mut stack,&stats,&traits,&[],false).unwrap();
        assert_eq!(stack.data().unwrap().other("minecraft:custom_data"),Some(&custom));
        assert_eq!(stack.data().unwrap().other("minecraft:tool"),Some(&tool));
        let progress = amount.max(0);
        let remainder = progress % 45;
        let logical = trait_level.max(0).checked_add(progress / 45).and_then(|whole| whole.checked_add(i32::from(remainder != 0)));
        let effective = logical.map(|logical| if remainder == 0 {logical as f32} else {logical as f32 + (remainder - 45) as f32 / 45.0}).unwrap_or(0.0);
        let expected = (0.1f32 + 4.0 * effective).clamp(0.1,2048.0);
        for _ in 0..2 {
            tconstruct_source_mining_stack::apply(&mut stack,&stats,&traits,&[],true).unwrap();
            let data = stack.data().unwrap();
            let custom = data.other("minecraft:custom_data").unwrap();
            assert_eq!(custom.get("fixture:foreign"),Some(&Tag::String("preserve".into())));
            assert_eq!(custom.get("tic_upgrades").and_then(|u|u.get("tconstruct:haste")),Some(&Tag::Int(amount)));
            assert_eq!(custom.get("tic_mining_tool"),data.other("minecraft:tool"));
            let Some(Tag::List(rules)) = data.other("minecraft:tool").and_then(|t|t.get("rules")) else {panic!("rules")};
            assert_eq!(rules[0].get("speed"),Some(&Tag::Float(expected.to_bits())));
            assert_eq!(tconstruct_material_mining_speed(1.0,Some(&stack),"minecraft:stone",&items).to_bits(),expected.max(1.0).to_bits());
            assert_eq!(tconstruct_material_mining_speed(1.0,Some(&stack),"minecraft:dirt",&items),1.0);
        }
        if [(0,1),(1,3),(2,6)].contains(&(trait_level,amount)) {
            let old = 0.1f32 + 4.0 * (amount as f32 / 45.0 + trait_level as f32);
            assert_ne!(expected.to_bits(),old.to_bits());
        }
        // Removing progress and traits rebuilds from source, not the last result.
        stack.data_mut().set_other("minecraft:custom_data",Some(Tag::compound([("tic_max_damage",Tag::Int(100))])));
        tconstruct_source_mining_stack::apply(&mut stack,&stats,&BTreeMap::new(),&[],true).unwrap();
        let Some(Tag::List(rules)) = stack.data().unwrap().other("minecraft:tool").and_then(|t|t.get("rules")) else {panic!("rules")};
        assert_eq!(rules[0].get("speed"),Some(&Tag::Float(0.1f32.to_bits())));
    }
}

#[path = "../../mixins/tconstruct/source-haste-wire.rs"] mod tconstruct_source_haste_wire;

#[test]
fn chest_haste_incremental_precision_and_existing_gates() {
    use tconstruct_source_mining_rules::haste_armor_multiplier_with_trait as multiplier;
    let custom = |progress| Tag::compound([
        ("tic_max_damage",Tag::Int(100)),
        ("tic_upgrades",Tag::compound([("tconstruct:haste",progress)])),
    ]);
    for (traits,progress) in [(0,30),(1,96),(2,14),(3,6),(0,0),(2,0),(2,44),(2,45),(2,46),(2,89),(2,90),(2,91),(-1,30),(2,-1),(0,i32::MAX),(i32::MAX,1),(i32::MAX,45)] {
        let amount=progress.max(0); let remainder=amount%45;
        let logical=traits.max(0).checked_add(amount/45).and_then(|n|n.checked_add(i32::from(remainder!=0)));
        let level=logical.map(|n|if remainder==0 {n as f32}else{n as f32+(remainder-45) as f32/45.0}).unwrap_or(0.0);
        let expected=1.0f32+0.1f32*level;
        let data=custom(Tag::Int(progress));
        assert_eq!(multiplier(&data,99,traits).to_bits(),expected.to_bits());
        assert_eq!(multiplier(&data,100,traits),1.0);
        assert_eq!(multiplier(&data,u32::MAX,traits),1.0);
        if [(0,30),(1,96),(2,14),(3,6)].contains(&(traits,progress)) {
            let old=1.0f32+0.1*(progress as f32/45.0+traits as f32);
            assert_ne!(expected.to_bits(),old.to_bits());
        }
    }
    for data in [Tag::compound(Vec::<(&str,Tag)>::new()),Tag::compound([("tic_max_damage",Tag::Int(0))]),Tag::compound([("tic_max_damage",Tag::Int(-1))]),Tag::compound([("tic_max_damage",Tag::Float(100.0f32.to_bits()))])] {
        assert_eq!(multiplier(&data,0,2),1.0);
    }
    for data in [custom(Tag::String("bad".into())),Tag::compound([("tic_max_damage",Tag::Int(100))])] {
        assert_eq!(multiplier(&data,0,2).to_bits(),1.2f32.to_bits());
    }
}

#[test]
fn chest_haste_wire_trait_and_local_trait_use_identical_level_math() {
    use tconstruct_source_haste_wire::{encode,decode,matching_level};
    use tconstruct_source_mining_rules::{haste_armor_multiplier_with_trait,finish_mining_speed};
    let mut chest=ItemStack::new("tconstruct:plate_chestplate",1);
    chest.data_mut().set_other("minecraft:custom_data",Some(Tag::compound([
        ("tic_max_damage",Tag::Int(100)),
        ("tic_upgrades",Tag::compound([("tconstruct:haste",Tag::Int(14))])),
    ])));
    let snapshot=decode(&encode(Some((&chest,2)))).unwrap();
    let evaluate=|chest:&ItemStack,level|haste_armor_multiplier_with_trait(chest.data().unwrap().other("minecraft:custom_data").unwrap(),chest.damage,level);
    let local=evaluate(&chest,2);
    let remote=evaluate(&chest,matching_level(Some(&snapshot),Some(&chest)).unwrap_or(0));
    assert_eq!(local.to_bits(),remote.to_bits());
    assert_eq!(finish_mining_speed(7.0,remote).to_bits(),(7.0f32*local).to_bits());
    // Missing/stale authoritative frames keep the existing upgrade-only fallback.
    assert_eq!(matching_level(None,Some(&chest)),None);
    chest.damage=1;
    assert_eq!(matching_level(Some(&snapshot),Some(&chest)),None);
    let fallback=evaluate(&chest,matching_level(Some(&snapshot),Some(&chest)).unwrap_or(0));
    let upgrade_only=1.0f32+0.1f32*(1.0f32+(14-45) as f32/45.0);
    assert_eq!(fallback.to_bits(),upgrade_only.to_bits());
    assert_ne!(fallback.to_bits(),local.to_bits());
    chest.damage=100;
    assert_eq!(evaluate(&chest,2),1.0);
    chest.damage=0;
    assert_eq!(evaluate(&chest,2).to_bits(),local.to_bits());
}

#[test]
fn chest_haste_flag_only_breakage_and_repair_preserve_other_gates() {
    use tconstruct_source_mining_rules::haste_armor_multiplier_with_trait as multiplier;
    let custom=|flag:Option<Tag>,max:Tag| {
        let mut entries=vec![("tic_max_damage",max),("tic_upgrades",Tag::compound([("tconstruct:haste",Tag::Int(14))]))];
        if let Some(flag)=flag {entries.push(("tic_broken",flag));}
        Tag::compound(entries)
    };
    let intact=custom(None,Tag::Int(100));
    for traits in [0,2] {
        let expected=multiplier(&intact,0,traits);
        assert!(expected>1.0);
        for flag in [1,-1,127,-128] {
            let broken=custom(Some(Tag::Byte(flag)),Tag::Int(100));
            let before=broken.clone();
            for damage in [0,99,100,u32::MAX] {assert_eq!(multiplier(&broken,damage,traits),1.0);}
            assert_eq!(broken,before);
        }
        // Only the established Byte representation marks this custom flag broken.
        for flag in [None,Some(Tag::Byte(0)),Some(Tag::Int(1)),Some(Tag::String("true".into()))] {
            let repaired=custom(flag,Tag::Int(100));
            assert_eq!(multiplier(&repaired,0,traits).to_bits(),expected.to_bits());
            assert_eq!(multiplier(&repaired,99,traits).to_bits(),expected.to_bits());
            assert_eq!(multiplier(&repaired,100,traits),1.0);
        }
        for max in [Tag::Int(0),Tag::Int(-1),Tag::String("100".into())] {
            assert_eq!(multiplier(&custom(Some(Tag::Byte(0)),max),0,traits),1.0);
        }
    }
}

#[test]
fn chest_haste_wire_broken_flag_suppresses_upgrade_and_trait() {
    use tconstruct_source_haste_wire::{encode,decode,matching_level};
    use tconstruct_source_mining_rules::haste_armor_multiplier_with_trait;
    let mut chest=ItemStack::new("tconstruct:plate_chestplate",1);
    let custom=|flag|Tag::compound([
        ("tic_max_damage",Tag::Int(100)),("tic_broken",Tag::Byte(flag)),
        ("tic_upgrades",Tag::compound([("tconstruct:haste",Tag::Int(14))])),
    ]);
    chest.data_mut().set_other("minecraft:custom_data",Some(custom(1)));
    let snapshot=decode(&encode(Some((&chest,2)))).unwrap();
    assert_eq!(matching_level(Some(&snapshot),Some(&chest)),Some(2));
    let evaluate=|item:&ItemStack,traits|haste_armor_multiplier_with_trait(item.data().unwrap().other("minecraft:custom_data").unwrap(),item.damage,traits);
    assert_eq!(evaluate(&chest,2),1.0);
    assert_eq!(evaluate(&snapshot.0,snapshot.1),1.0);
    assert_eq!(evaluate(&chest,matching_level(None,Some(&chest)).unwrap_or(0)),1.0);
    chest.data_mut().set_other("minecraft:custom_data",Some(custom(0)));
    assert_eq!(matching_level(Some(&snapshot),Some(&chest)),None);
    let repaired=decode(&encode(Some((&chest,2)))).unwrap();
    assert_eq!(matching_level(Some(&repaired),Some(&chest)),Some(2));
    assert!(evaluate(&chest,repaired.1)>1.0);
}

#[test]
fn conditional_mining_flag_only_breakage_skips_supported_hooks() {
    use tconstruct_source_mining_rules::{conditional_with_water,airborne_multiplier,finish_mining_speed};
    let custom=|id:&str,level,flag:Option<Tag>| {
        let mut entries=vec![
            ("tic_max_damage",Tag::Int(100)),("tic_overslime",Tag::Int(10)),
            ("tic_mining_traits",Tag::List(vec![Tag::compound([("id",Tag::String(id.into())),("level",Tag::Int(level))])])),
        ];
        if let Some(flag)=flag {entries.push(("tic_broken",flag));}
        Tag::compound(entries)
    };
    let run=|data:&Tag,damage,effective,not_required|conditional_with_water(data,damage,4.0,1.0,Some((0,320)),effective,not_required,Some(false),Some(3.0),Some(false));
    for (id,level) in [("tconstruct:jagged",1),("tconstruct:stonebound",1),("tconstruct:maintained",1),("tconstruct:dwarven",1),("tconstruct:crumbling",1),("tconstruct:blasting",20),("tconstruct:hydraulic",36),("tconstruct:overwield",1)] {
        let intact=custom(id,level,None);
        let expected=run(&intact,25,true,true).unwrap();
        assert_ne!(expected,4.0,"fixture must activate {id}");
        for flag in [1,-1,127,-128] {
            let broken=custom(id,level,Some(Tag::Byte(flag)));let before=broken.clone();
            for damage in [0,25,99,100] {
                for (effective,not_required) in [(true,true),(true,false),(false,true),(false,false)] {
                    assert_eq!(run(&broken,damage,effective,not_required),None);
                }
            }
            assert_eq!(broken,before);
            // The production caller retains host speed when the hook is absent.
            // Separate armor multiplication must not be suppressed by this tool.
            assert_eq!(finish_mining_speed(run(&broken,25,true,true).unwrap_or(4.0),1.2),4.0f32*1.2);
        }
        for flag in [None,Some(Tag::Byte(0)),Some(Tag::Int(1)),Some(Tag::String("true".into()))] {
            let repaired=custom(id,level,flag);
            assert_eq!(run(&repaired,25,true,true).unwrap().to_bits(),expected.to_bits());
            assert_eq!(run(&repaired,100,true,true),None);
            assert_eq!(run(&repaired,25,false,false),None);
        }
    }
    // Airborne has its own post-hook entry point and also works on ineffective tools.
    let intact=custom("tconstruct:airborne",1,None);
    assert_eq!(airborne_multiplier(&intact,25,false),5.0);
    for flag in [1,-1,127,-128] {
        let broken=custom("tconstruct:airborne",1,Some(Tag::Byte(flag)));
        assert_eq!(airborne_multiplier(&broken,25,false),1.0);
        assert_eq!(airborne_multiplier(&broken,25,true),1.0);
    }
    for flag in [None,Some(Tag::Byte(0)),Some(Tag::Int(1))] {
        let repaired=custom("tconstruct:airborne",1,flag);
        assert_eq!(airborne_multiplier(&repaired,25,false),5.0);
        assert_eq!(airborne_multiplier(&repaired,25,true),1.0);
        assert_eq!(airborne_multiplier(&repaired,100,false),1.0);
    }
}

include!("../../mixins/tconstruct/broken-harvest.rs");

#[test]
fn flag_only_broken_harvest_is_rejected_at_head_and_return() {
    use std::ops::ControlFlow::{Break,Continue};
    let items=Items::tags_only(std::path::Path::new("/tmp"),&horizon_world::loot::PackOverlay::NONE).unwrap();
    let mut stack=ItemStack::new("tconstruct:pickaxe",1);
    let data=|flag:Option<Tag>,max:Option<Tag>| {
        let mut entries=Vec::new();
        if let Some(flag)=flag {entries.push(("tic_broken",flag));}
        if let Some(max)=max {entries.push(("tic_max_damage",max));}
        Tag::compound(entries)
    };
    let tool=Tag::compound([("rules",Tag::List(vec![Tag::compound([
        ("blocks",Tag::String("minecraft:stone".into())),("correct_for_drops",Tag::Byte(1)),
    ])]))]);
    stack.data_mut().set_other("minecraft:tool",Some(tool));
    for flag in [1,-1,127,-128] {
        for max in [Some(Tag::Int(100)),Some(Tag::Int(0)),None,Some(Tag::String("100".into()))] {
            stack.data_mut().set_other("minecraft:custom_data",Some(data(Some(Tag::Byte(flag)),max)));
            let before=stack.clone();
            assert_eq!(broken_harvest(Some(&stack)),Break(false));
            for incoming in [false,true] {
                assert!(!tconstruct_material_correct_for_drops(incoming,Some(&stack),"minecraft:stone",&items));
            }
            assert!(stack.same_item_same_components(&before));
            assert_eq!(stack.damage,before.damage);
        }
    }
    for flag in [None,Some(Tag::Byte(0)),Some(Tag::Int(1)),Some(Tag::String("true".into()))] {
        stack.data_mut().set_other("minecraft:custom_data",Some(data(flag,Some(Tag::Int(100)))));
        stack.damage=99;
        assert_eq!(broken_harvest(Some(&stack)),Continue(()));
        assert!(tconstruct_material_correct_for_drops(false,Some(&stack),"minecraft:stone",&items));
        assert!(!tconstruct_material_correct_for_drops(true,Some(&stack),"minecraft:dirt",&items));
        stack.damage=100;
        assert_eq!(broken_harvest(Some(&stack)),Break(false));
        assert!(!tconstruct_material_correct_for_drops(true,Some(&stack),"minecraft:stone",&items));
        stack.damage=0;
    }
    // Namespace and absent-stack boundaries stay unchanged.
    let mut vanilla=ItemStack::new("minecraft:iron_pickaxe",1);
    vanilla.data_mut().set_other("minecraft:custom_data",Some(data(Some(Tag::Byte(1)),Some(Tag::Int(100)))));
    for held in [None,Some(&vanilla)] {
        assert_eq!(broken_harvest(held),Continue(()));
        assert!(tconstruct_material_correct_for_drops(true,held,"minecraft:stone",&items));
    }
    let mut flint=ItemStack::new("tconstruct:flint_pickaxe",1);
    flint.damage=84;assert_eq!(broken_harvest(Some(&flint)),Continue(()));
    flint.damage=85;assert_eq!(broken_harvest(Some(&flint)),Break(false));
}

include!("../../mixins/tconstruct/broken-speed.rs");

#[test]
fn flag_only_broken_mining_speed_is_preserved_at_head_and_return() {
    use std::ops::ControlFlow::{Break,Continue};
    let items=Items::tags_only(std::path::Path::new("/tmp"),&horizon_world::loot::PackOverlay::NONE).unwrap();
    let mut stack=ItemStack::new("tconstruct:pickaxe",1);
    let data=|flag:Option<Tag>,max:Option<Tag>| {
        let mut entries=vec![("tic_mining_traits",Tag::List(vec![]))];
        if let Some(flag)=flag {entries.push(("tic_broken",flag));}
        if let Some(max)=max {entries.push(("tic_max_damage",max));}
        Tag::compound(entries)
    };
    let tool=Tag::compound([("rules",Tag::List(vec![Tag::compound([
        ("blocks",Tag::String("minecraft:stone".into())),("speed",Tag::Float(7.0f32.to_bits())),
    ])]))]);
    for flag in [1,-1,127,-128] {
        for max in [Some(Tag::Int(100)),Some(Tag::Int(0)),None,Some(Tag::String("100".into()))] {
            stack.data_mut().set_other("minecraft:custom_data",Some(data(Some(Tag::Byte(flag)),max)));
            for tool in [Some(tool.clone()),None] {
                stack.data_mut().set_other("minecraft:tool",tool);
                let before=stack.clone();
                for damage in [0,99,100] {
                    stack.damage=damage;
                    assert_eq!(broken_speed(Some(&stack)),Break(0.3));
                    for block in ["minecraft:stone","minecraft:dirt"] {
                        for incoming in [0.3,1.0,7.0] {
                            assert_eq!(tconstruct_material_mining_speed(incoming,Some(&stack),block,&items).to_bits(),0.3f32.to_bits());
                        }
                    }
                }
                stack.damage=before.damage;
                assert!(stack.same_item_same_components(&before));
            }
            stack.damage=0;
        }
    }
    stack.data_mut().set_other("minecraft:tool",Some(tool));
    for flag in [None,Some(Tag::Byte(0)),Some(Tag::Int(1)),Some(Tag::String("true".into()))] {
        stack.data_mut().set_other("minecraft:custom_data",Some(data(flag,Some(Tag::Int(100)))));
        assert_eq!(broken_speed(Some(&stack)),Continue(()));
        assert_eq!(tconstruct_material_mining_speed(0.3,Some(&stack),"minecraft:stone",&items),7.0);
        assert_eq!(tconstruct_material_mining_speed(0.3,Some(&stack),"minecraft:dirt",&items),1.0);
        stack.damage=100;
        assert_eq!(broken_speed(Some(&stack)),Break(0.3));
        // Existing damage-limit RETURN fallback still delegates a Tool-less clone.
        let mut fallback=stack.clone();fallback.data_mut().set_other("minecraft:tool",None);
        assert_eq!(tconstruct_material_mining_speed(0.3,Some(&stack),"minecraft:stone",&items),items.mining_speed(Some(&fallback),"minecraft:stone"));
        stack.damage=0;
    }
    let mut vanilla=ItemStack::new("minecraft:iron_pickaxe",1);
    vanilla.data_mut().set_other("minecraft:custom_data",Some(data(Some(Tag::Byte(1)),Some(Tag::Int(100)))));
    for held in [None,Some(&vanilla)] {
        assert_eq!(broken_speed(held),Continue(()));
        assert_eq!(tconstruct_material_mining_speed(7.0,held,"minecraft:stone",&items),7.0);
    }
    let mut flint=ItemStack::new("tconstruct:flint_pickaxe",1);
    flint.damage=84;assert_eq!(broken_speed(Some(&flint)),Continue(()));
    flint.damage=85;assert_eq!(broken_speed(Some(&flint)),Break(0.3));
}
