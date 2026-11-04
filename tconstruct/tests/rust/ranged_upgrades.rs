extern crate horizon_items as horizon_client;
#[path = "../../mixins/tconstruct/source-fluid-display.rs"] mod tconstruct_source_fluid_display;
#[path = "../../../pluto-lib/mixins/pluto_native_creative/stack-fluid.rs"] mod pluto_stack_fluid;
#[path = "../../mixins/tconstruct/source-smeltery-tank.rs"] mod tconstruct_source_smeltery_tank;
#[path = "../../mixins/tconstruct/source-melter-state.rs"] mod tconstruct_source_melter_state;
#[path = "../../mixins/tconstruct/source-smeltery-fuel.rs"] mod tconstruct_source_smeltery_fuel;
#[path = "../../mixins/tconstruct/source-fluid-container.rs"] mod tconstruct_source_fluid_container;
#[path = "../../mixins/tconstruct/source-dwarven.rs"] mod tconstruct_source_dwarven;
#[path = "../../mixins/tconstruct/source-station-stats.rs"] mod tconstruct_source_station_stats;
#[path = "../../mixins/tconstruct/source-melee-stat-ops.rs"] mod tconstruct_source_melee_stat_ops;
#[path = "../../mixins/tconstruct/source-material-effects.rs"] mod tconstruct_source_material_effects;
#[path = "../../mixins/tconstruct/source-upgrade-rules.rs"] mod tconstruct_source_upgrade_rules;
#[path = "../../mixins/tconstruct/source-ranged-draw.rs"] mod draw;
#[path = "../../mixins/tconstruct/source-projectile-stats.rs"] mod projectile;
#[test]
fn modified_stats_change_real_draw_and_projectile_spread() {
 use horizon_items::items::ItemStack; use horizon_world::nbt::Tag; use std::collections::BTreeMap;
 let def = br#"{"modules":[{"type":"tconstruct:base_stats","stats":{"tconstruct:draw_speed":1.0,"tconstruct:velocity":1.0,"tconstruct:accuracy":0.75}}]}"#;
 let stats=tconstruct_source_station_stats::base_stats(def,&[]).unwrap();
 let mut bow=ItemStack::new("tconstruct:crossbow",1);
 bow.data_mut().set_other("minecraft:custom_data",Some(Tag::compound([("tic_draw_speed",Tag::Float(1.0f32.to_bits())),("tic_velocity",Tag::Float(1.0f32.to_bits())),("tic_accuracy",Tag::Float(0.75f32.to_bits())),("tic_upgrades",Tag::compound([("tconstruct:quick_charge",Tag::Int(5)),("tconstruct:trueshot",Tag::Int(10))]))])));
 let before=(draw::draw_ticks(&bow),projectile::projectile_stats(&bow).unwrap().1);
 tconstruct_source_upgrade_rules::apply_stats(&mut bow,&stats,&BTreeMap::new(),false,false).unwrap();
 let after=(draw::draw_ticks(&bow),projectile::projectile_stats(&bow).unwrap().1);
 assert_eq!(before.0,20); assert_eq!(after.0,16); assert!(after.1<before.1);
}

#[test]
fn source_ranged_traits_combine_with_upgrades_and_reach_firing_stats() {
 use horizon_items::items::ItemStack; use horizon_world::nbt::Tag; use std::collections::BTreeMap;
 let def=br#"{"modules":[{"type":"tconstruct:base_stats","stats":{"tconstruct:draw_speed":1.0,"tconstruct:velocity":1.0,"tconstruct:accuracy":0.75}}]}"#;
 let stats=tconstruct_source_station_stats::base_stats(def,&[]).unwrap();
 let mut bow=ItemStack::new("tconstruct:crossbow",1);
 bow.data_mut().set_other("minecraft:custom_data",Some(Tag::compound([("tic_upgrades",Tag::compound([
  ("tconstruct:quick_charge",Tag::Int(5)),("tconstruct:trueshot",Tag::Int(10)),("tconstruct:diamond",Tag::Int(1)),
  ("tconstruct:emerald",Tag::Int(1)),("tconstruct:netherite",Tag::Int(1)),
 ]))])));
 let traits=BTreeMap::from([("tconstruct:lightweight".into(),2),("tconstruct:featherweight".into(),1),
  ("tconstruct:heavy".into(),1),("tconstruct:vintage".into(),1),("tconstruct:entangled".into(),1),
  ("tconstruct:crystalbound".into(),1),("tconstruct:ductile".into(),2)]);
 tconstruct_source_upgrade_rules::apply_stats(&mut bow,&stats,&traits,false,false).unwrap();
 let data=bow.data().unwrap().other("minecraft:custom_data").unwrap();
 let number=|key| match data.get(key) {Some(Tag::Float(bits))=>f32::from_bits(*bits),_=>panic!("missing {key}")};
 assert!((number("tic_draw_speed")-1.4175).abs()<0.0001);
 assert!((number("tic_velocity")-1.276).abs()<0.0001);
 assert_eq!(number("tic_accuracy"),1.0);
 assert_eq!(draw::draw_ticks(&bow),15);
 assert!((projectile::projectile_stats(&bow).unwrap().0-1.276).abs()<0.0001);
 assert_eq!(projectile::projectile_bonus(&bow),1.0);
 assert_eq!(projectile::crystalbound_level(&bow),1);
 tconstruct_source_upgrade_rules::apply_stats(&mut bow,&stats,&traits,false,false).unwrap();
 assert_eq!(projectile::projectile_bonus(&bow),1.0);
}

#[test]
fn source_additive_traits_respect_definition_stat_multipliers() {
 use horizon_items::items::ItemStack; use horizon_world::nbt::Tag; use std::collections::BTreeMap;
 let def=br#"{"modules":[{"type":"tconstruct:base_stats","stats":{"tconstruct:draw_speed":1.0,"tconstruct:velocity":1.0,"tconstruct:accuracy":0.5}},{"type":"tconstruct:multiply_stats","multipliers":{"tconstruct:draw_speed":2.0,"tconstruct:velocity":3.0,"tconstruct:accuracy":0.5,"tconstruct:projectile_damage":4.0}}]}"#;
 let stats=tconstruct_source_station_stats::base_stats(def,&[]).unwrap();let mut bow=ItemStack::new("tconstruct:longbow",1);
 bow.data_mut().set_other("minecraft:custom_data",Some(Tag::compound(Vec::<(&str,Tag)>::new())));
 let levels=BTreeMap::from([("tconstruct:lightweight".into(),1),("tconstruct:heavy".into(),1),("tconstruct:ductile".into(),1)]);
 tconstruct_source_upgrade_rules::apply_stats(&mut bow,&stats,&levels,false,false).unwrap();
 let data=bow.data().unwrap().other("minecraft:custom_data").unwrap();
 let n=|key| match data.get(key){Some(Tag::Float(v))=>f32::from_bits(*v),_=>panic!("missing {key}")};
 assert!((n("tic_draw_speed")-2.06).abs()<0.0001);
 assert!((n("tic_velocity")-3.09).abs()<0.0001);
 assert!((n("tic_accuracy")-0.3).abs()<0.0001);
 assert_eq!(projectile::projectile_bonus(&bow),7.0);
}

#[test]
fn maintained_rebuild_without_stacking_and_clear_after_part_swap() {
 use horizon_items::items::ItemStack; use horizon_world::nbt::Tag; use std::collections::BTreeMap;
 let def=br#"{"modules":[{"type":"tconstruct:base_stats","stats":{"tconstruct:velocity":1.0,"tconstruct:accuracy":1.0,"tconstruct:projectile_damage":2.0}},{"type":"tconstruct:multiply_stats","multipliers":{"tconstruct:velocity":2.0,"tconstruct:projectile_damage":3.0}}]}"#;
 let stats=tconstruct_source_station_stats::base_stats(def,&[]).unwrap();
 for id in ["tconstruct:longbow", "tconstruct:crossbow"] {
  let mut bow=ItemStack::new(id,1);
  bow.data_mut().set_other("minecraft:custom_data",Some(Tag::compound([
   ("tic_max_damage",Tag::Int(100)),
   ("tic_upgrades",Tag::compound([("tconstruct:diamond",Tag::Int(1)),("tconstruct:netherite",Tag::Int(1))])),
  ])));
  let levels=BTreeMap::from([("tconstruct:ductile".into(),1),("tconstruct:maintained".into(),2)]);
  for _ in 0..2 {
   tconstruct_source_upgrade_rules::apply_stats(&mut bow,&stats,&levels,false,false).unwrap();
   // Existing projectile damage remains base 6 + (diamond .5 + ductile .25) * 3.
   assert!((projectile::projectile_bonus(&bow)-6.25).abs()<0.00001);
   // Base 2 * Netherite 1.1 + Maintained .16 * definition multiplier 2.
   assert!((projectile::projectile_stats(&bow).unwrap().0-2.52).abs()<0.00001);
  }
  bow.damage=50;
  assert!((projectile::projectile_stats(&bow).unwrap().0-2.2).abs()<0.00001);
  tconstruct_source_upgrade_rules::apply_stats(&mut bow,&stats,&BTreeMap::new(),false,false).unwrap();
  bow.damage=0;
  assert!((projectile::projectile_bonus(&bow)-5.5).abs()<0.00001);
  assert!((projectile::projectile_stats(&bow).unwrap().0-2.2).abs()<0.00001);
 }
}

#[test]
fn maintained_excludes_base_velocity_percent_from_conditional_bonus() {
 use horizon_items::items::ItemStack; use horizon_world::nbt::Tag; use std::collections::BTreeMap;
 let def=br#"{"modules":[{"type":"tconstruct:base_stats","stats":{"tconstruct:velocity":1.0,"tconstruct:accuracy":1.0}},{"type":"tconstruct:multiply_stats","multipliers":{"tconstruct:velocity":2.0}}]}"#;
 let mut stats=tconstruct_source_station_stats::base_stats(def,&[]).unwrap();
 stats.velocity_raw[2]=0.5; // A base-stat percent must not scale the conditional bonus.
 for id in ["tconstruct:longbow","tconstruct:crossbow"] {
  let mut bow=ItemStack::new(id,1);
  bow.data_mut().set_other("minecraft:custom_data",Some(Tag::compound([("tic_max_damage",Tag::Int(100))])));
  tconstruct_source_upgrade_rules::apply_stats(&mut bow,&stats,&BTreeMap::from([("tconstruct:maintained".into(),1)]),false,false).unwrap();
  // Base 1 * 1.5 * 2 plus .08 * 2, not .08 * 1.5 * 2.
  assert!((projectile::projectile_stats(&bow).unwrap().0-3.16).abs()<1e-6);
 }
}

#[test]
fn airborne_trait_rebuild_and_removal_changes_firing_velocity() {
 use horizon_items::items::ItemStack; use horizon_world::nbt::Tag; use std::collections::BTreeMap;
 let def=br#"{"modules":[{"type":"tconstruct:base_stats","stats":{"tconstruct:velocity":1.0,"tconstruct:accuracy":1.0}}]}"#;
 let stats=tconstruct_source_station_stats::base_stats(def,&[]).unwrap();
 for id in ["tconstruct:longbow", "tconstruct:crossbow"] {
  let mut bow=ItemStack::new(id,1);
  bow.data_mut().set_other("minecraft:custom_data",Some(Tag::compound(Vec::<(&str,Tag)>::new())));
  let levels=BTreeMap::from([("tconstruct:airborne".into(),1)]);
  tconstruct_source_upgrade_rules::apply_stats(&mut bow,&stats,&levels,false,false).unwrap();
  assert!((projectile::projectile_stats_at_airborne(&bow,64.0,320,true).unwrap().0-1.1).abs()<1e-6);
  tconstruct_source_upgrade_rules::apply_stats(&mut bow,&stats,&BTreeMap::new(),false,false).unwrap();
  assert!((projectile::projectile_stats_at_airborne(&bow,64.0,320,true).unwrap().0-1.0).abs()<1e-6);
 }
}


#[test]
fn spiny_velocity_and_removal_on_both_launchers() {
 use horizon_items::items::ItemStack; use horizon_world::nbt::Tag; use std::collections::BTreeMap;
 let def=br#"{"modules":[{"type":"tconstruct:base_stats","stats":{"tconstruct:velocity":1.0}},{"type":"tconstruct:multiply_stats","multipliers":{"tconstruct:velocity":2.0}}]}"#;
 let stats=tconstruct_source_station_stats::base_stats(def,&[]).unwrap();
 for id in ["tconstruct:longbow","tconstruct:crossbow"] {
  let mut tool=ItemStack::new(id,1);
  tool.data_mut().set_other("minecraft:custom_data",Some(Tag::compound([("tic_upgrades",Tag::compound([("tconstruct:netherite",Tag::Int(1))]))])));
  for (level,expected) in [(0,2.2_f32),(1,2.42),(2,2.64),(0,2.2)] {
   let traits=BTreeMap::from([("tconstruct:spiny".to_string(),level)]);
   tconstruct_source_upgrade_rules::apply_stats(&mut tool,&stats,&traits,false,false).unwrap();
   let v=projectile::projectile_stats(&tool).unwrap().0;
   assert!((v-expected).abs()<0.0001,"{id} level {level}: {v} != {expected}");
  }
 }
}


#[test]
fn blindshot_accepts_supported_launchers_and_rejects_other_items() {
 use horizon_items::items::ItemStack; use std::collections::{BTreeMap,HashMap,HashSet};
 let recipe=br#"{"type":"tconstruct:incremental_modifier","amount_per_item":1,"needed_per_level":10,"input":{"item":"minecraft:dirt"},"result":"tconstruct:blindshot","tools":{"tag":"tconstruct:modifiable/ranged"}}"#.to_vec();
 let recipes=BTreeMap::from([("blindshot".to_string(),recipe)]);
 let tags=HashMap::from([("tconstruct:modifiable/ranged".to_string(),HashSet::from(["tconstruct:longbow".to_string(),"tconstruct:crossbow".to_string(),"tconstruct:slime_sling".to_string()]))]);
 for id in ["tconstruct:longbow","tconstruct:crossbow"] {
  let tool=ItemStack::new(id,1);
  let input=[Some(ItemStack::new("minecraft:dirt",1))];
  let plan=tconstruct_source_upgrade_rules::plan(&recipes,&tags,&tool,&input,0).unwrap();
  assert_eq!(plan.id,"tconstruct:blindshot"); assert_eq!(plan.amount,1);
  assert!(tconstruct_source_upgrade_rules::plan(&recipes,&tags,&tool,&[Some(ItemStack::new("minecraft:stone",1))],0).is_none());
 }
 let sling=ItemStack::new("tconstruct:slime_sling",1);
 assert!(tconstruct_source_upgrade_rules::plan(&recipes,&tags,&sling,&[Some(ItemStack::new("minecraft:dirt",1))],0).is_none());
}


#[test]
fn blindshot_partial_levels_rebuild_and_clamp_without_slots() {
 use horizon_items::items::ItemStack; use horizon_world::nbt::Tag; use std::collections::BTreeMap;
 let def=br#"{"modules":[{"type":"tconstruct:base_stats","stats":{"tconstruct:accuracy":0.8}}]}"#;
 let stats=tconstruct_source_station_stats::base_stats(def,&[]).unwrap();
 for (points,expected) in [(0,0.8_f32),(1,0.79),(10,0.7),(100,0.1)] {
  let mut tool=ItemStack::new("tconstruct:longbow",1);
  tool.data_mut().set_other("minecraft:custom_data",Some(Tag::compound([("tic_upgrades",Tag::compound([("tconstruct:blindshot",Tag::Int(points))]))])));
  assert!(tconstruct_source_upgrade_rules::valid_slots(&tool,&BTreeMap::new(),0,0));
  assert!(!tconstruct_source_upgrade_rules::extractable(&tool).contains(&"tconstruct:blindshot"));
  for _ in 0..2 {
   tconstruct_source_upgrade_rules::apply_stats(&mut tool,&stats,&BTreeMap::new(),false,false).unwrap();
   let d=tool.data().unwrap().other("minecraft:custom_data").unwrap();
   let v=match d.get("tic_accuracy"){Some(Tag::Float(bits))=>f32::from_bits(*bits),_=>panic!("missing accuracy")};
   assert!((v-expected).abs()<0.0001,"{points}: {v} != {expected}");
  }
 }
}


#[test]
fn power_incremental_damage_rebuild_is_stable() {
 use horizon_items::items::ItemStack; use horizon_world::nbt::Tag; use std::collections::BTreeMap;
 let def=br#"{"modules":[{"type":"tconstruct:base_stats","stats":{"tconstruct:projectile_damage":2.0}},{"type":"tconstruct:multiply_stats","multipliers":{"tconstruct:projectile_damage":2.0}}]}"#;
 let stats=tconstruct_source_station_stats::base_stats(def,&[]).unwrap();
 for (points,expected) in [(0,4.0_f32),(36,5.5),(72,6.0)] {
  let mut tool=ItemStack::new("tconstruct:longbow",1);
  tool.data_mut().set_other("minecraft:custom_data",Some(Tag::compound([("tic_upgrades",Tag::compound([("tconstruct:power",Tag::Int(points))]))])));
  for _ in 0..2 {
   tconstruct_source_upgrade_rules::apply_stats(&mut tool,&stats,&BTreeMap::new(),false,false).unwrap();
   let v=projectile::projectile_bonus(&tool) as f32 + 2.0;
   assert!((v-expected).abs()<0.0001,"{points}: {v} != {expected}");
  }
 }
}


#[test]
fn overwield_uses_current_capacity_for_draw_and_velocity() {
 use horizon_items::items::ItemStack; use horizon_world::nbt::Tag;
 for id in ["tconstruct:longbow","tconstruct:crossbow"] {
  for (amount,expected_velocity,expected_ticks) in [(0,1.0_f32,20),(1,1.06,19),(3,1.12,18)] {
   let mut tool=ItemStack::new(id,1);
   tool.data_mut().set_other("minecraft:custom_data",Some(Tag::compound([
    ("tic_velocity",Tag::Float(1.0_f32.to_bits())),("tic_accuracy",Tag::Float(1.0_f32.to_bits())),
    ("tic_draw_speed",Tag::Float(1.0_f32.to_bits())),("tic_overwield_ranged",Tag::Int(2)),
    ("tic_overslime",Tag::Int(amount)),("tic_overwield_velocity_multiplier",Tag::Float(1.0_f32.to_bits())),
    ("tic_overwield_draw_multiplier",Tag::Float(1.0_f32.to_bits()))])));
   let actual=projectile::projectile_stats_at(&tool,64.0,256).unwrap().0;
   assert!((actual-expected_velocity).abs()<0.0001,"{id} amount {amount}: {actual}");
   assert_eq!(draw::draw_ticks(&tool),expected_ticks);
  }
 }
}


#[test]
fn blindshot_stays_unsalvageable_when_gilded_is_present() {
 use horizon_items::items::ItemStack; use horizon_world::nbt::Tag;
 let mut tool=ItemStack::new("tconstruct:longbow",1);
 tool.data_mut().set_other("minecraft:custom_data",Some(Tag::compound([("tic_upgrades",Tag::compound([
  ("tconstruct:blindshot",Tag::Int(10)),("tconstruct:gilded",Tag::Int(1))]))])));
 let ids=tconstruct_source_upgrade_rules::extractable(&tool);
 assert!(ids.contains(&"tconstruct:gilded"));
 assert!(!ids.contains(&"tconstruct:blindshot"));
}

#[path = "../../mixins/tconstruct/source-mining-stack.rs"] mod tconstruct_source_mining_stack;

#[test]
fn incremental_upgrades_merge_with_material_trait_levels() {
 use horizon_items::items::ItemStack; use horizon_world::nbt::Tag; use std::collections::BTreeMap;
 let def=br#"{"modules":[{"type":"tconstruct:base_stats","stats":{"tconstruct:mining_speed":3.0,"tconstruct:attack_damage":2.0,"tconstruct:attack_speed":1.0,"tconstruct:draw_speed":1.0,"tconstruct:accuracy":0.5}}]}"#;
 let stats=tconstruct_source_station_stats::base_stats(def,&[]).unwrap();
 let traits=BTreeMap::from([("tconstruct:haste".into(),2),("tconstruct:sharpness".into(),2),("tconstruct:swiftstrike".into(),2),("tconstruct:quick_charge".into(),2),("tconstruct:trueshot".into(),2)]);
 let mut pick=ItemStack::new("tconstruct:pickaxe",1);
 pick.data_mut().set_other("minecraft:custom_data",Some(Tag::compound([("tic_upgrades",Tag::compound([("tconstruct:haste",Tag::Int(18)),("tconstruct:sharpness",Tag::Int(18)),("tconstruct:swiftstrike",Tag::Int(36))]))])));
 pick.data_mut().set_other("minecraft:tool",Some(Tag::compound([("rules",Tag::List(vec![Tag::compound([("speed",Tag::Float(3.0f32.to_bits()))])]))])));
 for _ in 0..2 {
  tconstruct_source_mining_stack::apply(&mut pick,&stats,&traits,&[],true).unwrap();
  tconstruct_source_upgrade_rules::apply_stats(&mut pick,&stats,&traits,true,true).unwrap();
  let data=pick.data().unwrap(); let n=|key| match data.other("minecraft:custom_data").unwrap().get(key){Some(Tag::Float(v))=>f32::from_bits(*v),_=>panic!("{key}")};
  let Tag::List(rules)=data.other("minecraft:tool").unwrap().get("rules").unwrap() else { panic!("rules") };
  let speed=rules[0].get("speed").unwrap();
  assert!(matches!(speed,Tag::Float(v) if (f32::from_bits(*v)-12.6).abs()<1e-5));
  assert!((n("tic_attack_damage")-3.875).abs()<1e-5);
  assert!((n("tic_attack_speed")-1.125).abs()<1e-5);
 }
 // A changed material recipe removes the trait contribution rather than
 // stacking it with the previous rebuild's output.
 tconstruct_source_mining_stack::apply(&mut pick,&stats,&BTreeMap::new(),&[],true).unwrap();
 tconstruct_source_upgrade_rules::apply_stats(&mut pick,&stats,&BTreeMap::new(),true,true).unwrap();
 let data=pick.data().unwrap();
 let Tag::List(rules)=data.other("minecraft:tool").unwrap().get("rules").unwrap() else { panic!("rules") };
 assert!(matches!(rules[0].get("speed"),Some(Tag::Float(v)) if (f32::from_bits(*v)-4.6).abs()<1e-5));
 let custom=data.other("minecraft:custom_data").unwrap();
 assert!(matches!(custom.get("tic_attack_damage"),Some(Tag::Float(v)) if (f32::from_bits(*v)-2.375).abs()<1e-5));
 assert!(matches!(custom.get("tic_attack_speed"),Some(Tag::Float(v)) if (f32::from_bits(*v)-1.025).abs()<1e-5));
 for id in ["tconstruct:longbow","tconstruct:crossbow"] {
  let mut bow=ItemStack::new(id,1);
  bow.data_mut().set_other("minecraft:custom_data",Some(Tag::compound([("tic_upgrades",Tag::compound([("tconstruct:quick_charge",Tag::Int(2)),("tconstruct:trueshot",Tag::Int(5))]))])));
  tconstruct_source_upgrade_rules::apply_stats(&mut bow,&stats,&traits,false,false).unwrap();
  let data=bow.data().unwrap().other("minecraft:custom_data").unwrap();let n=|key| match data.get(key){Some(Tag::Float(v))=>f32::from_bits(*v),_=>panic!("{key}")};
  assert!((n("tic_draw_speed")-1.6).abs()<1e-5);
  assert!((n("tic_accuracy")-0.75).abs()<1e-5);
 }
}

#[test]
fn incremental_trait_only_upgrade_only_and_removal_rebuild() {
 use horizon_items::items::ItemStack; use horizon_world::nbt::Tag; use std::collections::BTreeMap;
 let def=br#"{"modules":[{"type":"tconstruct:base_stats","stats":{"tconstruct:attack_damage":2,"tconstruct:attack_speed":1,"tconstruct:draw_speed":1,"tconstruct:accuracy":0.5}}]}"#;
 let stats=tconstruct_source_station_stats::base_stats(def,&[]).unwrap();
 let mut bow=ItemStack::new("tconstruct:crossbow",1);
 bow.data_mut().set_other("minecraft:custom_data",Some(Tag::compound([("tic_upgrades",Tag::compound([("tconstruct:quick_charge",Tag::Int(2)),("tconstruct:trueshot",Tag::Int(5))]))])));
 let levels=BTreeMap::from([("tconstruct:quick_charge".into(),2),("tconstruct:trueshot".into(),2)]);
 for (traits,draw,accuracy) in [(&levels,1.6,0.75),(&BTreeMap::new(),1.1,0.55)] {
  tconstruct_source_upgrade_rules::apply_stats(&mut bow,&stats,traits,false,false).unwrap();
  let data=bow.data().unwrap().other("minecraft:custom_data").unwrap();let n=|key| match data.get(key){Some(Tag::Float(v))=>f32::from_bits(*v),_=>panic!("{key}")};
  assert!((n("tic_draw_speed")-draw).abs()<1e-5);assert!((n("tic_accuracy")-accuracy).abs()<1e-5);
 }
 bow.data_mut().set_other("minecraft:custom_data",Some(Tag::compound(Vec::<(&str,Tag)>::new())));
 tconstruct_source_upgrade_rules::apply_stats(&mut bow,&stats,&levels,false,false).unwrap();
 let data=bow.data().unwrap().other("minecraft:custom_data").unwrap();
 assert!(matches!(data.get("tic_draw_speed"),Some(Tag::Float(v)) if (f32::from_bits(*v)-1.5).abs()<1e-5));
 assert!(matches!(data.get("tic_accuracy"),Some(Tag::Float(v)) if (f32::from_bits(*v)-0.7).abs()<1e-5));
}

#[path = "../../mixins/tconstruct/source-mining-math.rs"] mod tconstruct_source_mining_math;
#[path = "../../mixins/tconstruct/source-mining-rules.rs"] mod tconstruct_source_mining_rules;
#[path = "../../../pluto-lib/mixins/pluto_tool_rules/rules.rs"] pub mod rules;
mod tconstruct_tool_api {
    pub use horizon_items::item_data::canon_tag;
    pub use crate::rules;
}

use horizon_items::items::ItemStack;

#[test]
fn local_armor_haste_survives_item_codec_and_unequip_has_no_effect() {
 use horizon_world::nbt::Tag;
 let mut chest=ItemStack::new("tconstruct:plate_chestplate",1);
 chest.data_mut().set_other("minecraft:custom_data",Some(Tag::compound([
  ("tic_max_damage",Tag::Int(100)),
  ("tic_materials",Tag::List(vec![Tag::String("test:haste_material".into())])),
  ("tic_upgrades",Tag::compound([("tconstruct:haste",Tag::Int(18))])),
 ])));
 let encoded=horizon_items::item_nbt::item_to_nbt(&chest);
 let restored=horizon_items::item_nbt::item_from_nbt(&encoded).unwrap();
 assert!(chest.same_item_same_components(&restored));
 let worn=|chest: Option<&ItemStack>, trait_level| chest.and_then(|item| {
  let custom=item.data()?.other("minecraft:custom_data")?;
  Some(tconstruct_source_mining_rules::haste_armor_multiplier_with_trait(custom,item.damage,trait_level))
 }).unwrap_or(1.0);
 assert!((worn(Some(&restored),2)-1.24).abs()<1e-6);
 assert_eq!(worn(None,2),1.0);
 assert!((worn(Some(&restored),0)-1.04).abs()<1e-6);
}

#[path = "../../mixins/tconstruct/source-haste-wire.rs"] mod tconstruct_source_haste_wire;

#[path = "../../mixins/tconstruct/source-haste-source.rs"] mod tconstruct_source_haste_source;

#[test]
fn ambush_projectile_tracks_shooter_full_health_and_material_rebuild() {
 use horizon_items::items::ItemStack; use horizon_world::nbt::Tag; use std::collections::BTreeMap;
 let def=br#"{"modules":[{"type":"tconstruct:base_stats","stats":{"tconstruct:draw_speed":1.0,"tconstruct:velocity":1.0,"tconstruct:accuracy":1.0}},{"type":"tconstruct:multiply_stats","multipliers":{"tconstruct:projectile_damage":2.0}}]}"#;
 let stats=tconstruct_source_station_stats::base_stats(def,&[]).unwrap();
 for id in ["tconstruct:longbow", "tconstruct:crossbow"] {
  let mut bow=ItemStack::new(id,1);
  bow.data_mut().set_other("minecraft:custom_data",Some(Tag::compound(Vec::<(&str,Tag)>::new())));
  let traits=BTreeMap::from([("tconstruct:ambush".into(),2)]);
  tconstruct_source_upgrade_rules::apply_stats(&mut bow,&stats,&traits,false,false).unwrap();
  let base=projectile::projectile_bonus(&bow);
  let run=|hp,max| projectile::projectile_bonus_at(&bow,hp,max);
  assert!((run(Some(20.0),Some(20.0))-base-2.0).abs()<1e-6);
  assert!((run(Some(19.01),Some(20.0))-base-2.0).abs()<1e-6);
  assert_eq!(run(Some(19.0),Some(20.0)),base);
  assert!((run(Some(29.1),Some(30.0))-base-2.0).abs()<1e-6);
  assert_eq!(run(Some(20.0),Some(30.0)),base);
  assert_eq!(run(Some(20.0),None),base);
  assert_eq!(run(Some(20.0),Some(f32::NAN)),base);
  assert_eq!(run(None,Some(20.0)),base);
  let combined=BTreeMap::from([("tconstruct:ambush".into(),2), ("tconstruct:raging".into(),1)]);
  tconstruct_source_upgrade_rules::apply_stats(&mut bow,&stats,&combined,false,false).unwrap();
  let with_both=projectile::projectile_bonus_at(&bow,Some(9.1),Some(10.0));
  assert!((with_both-base-2.38).abs()<1e-5);
  let without_max=projectile::projectile_bonus_at(&bow,Some(9.1),None);
  assert!((without_max-base-0.38).abs()<1e-5);
  tconstruct_source_upgrade_rules::apply_stats(&mut bow,&stats,&BTreeMap::new(),false,false).unwrap();
  assert_eq!(projectile::projectile_bonus_at(&bow,Some(20.0),Some(20.0)),base);
 }
}

#[test]
fn ambush_and_raging_preserve_trait_order_at_float_rounding_boundary() {
 use horizon_items::items::ItemStack; use horizon_world::nbt::Tag;
 let run = |order: &[&str]| {
  let mut bow = ItemStack::new("tconstruct:longbow", 1);
  bow.data_mut().set_other("minecraft:custom_data", Some(Tag::compound([
   ("tic_projectile_damage", Tag::Float(2.0f32.to_bits())),
   ("tic_raging_projectile_multiplier", Tag::Float(1.1f32.to_bits())),
   ("tic_ambush_projectile", Tag::Int(1)), ("tic_raging", Tag::Int(1)),
   ("tic_trait_order", Tag::List(order.iter().map(|id| Tag::String((*id).into())).collect())),
  ])));
  projectile::projectile_bonus_at(&bow, Some(1.01), Some(2.0)) as f32
 };
 let health = 1.01f32; let multiplier = 1.1f32;
 let ambush = 0.5f32 * multiplier;
 let raging = ((11.0f32 - health) / 10.0).clamp(0.0, 1.0) * multiplier;
 let ambush_first = (2.0f32 + ambush + raging).clamp(0.0, 1024.0) - 2.0;
 let raging_first = (2.0f32 + raging + ambush).clamp(0.0, 1024.0) - 2.0;
 assert_ne!(ambush_first.to_bits(), raging_first.to_bits());
 assert_eq!(run(&["tconstruct:ambush", "tconstruct:raging"]).to_bits(), ambush_first.to_bits());
 assert_eq!(run(&["tconstruct:raging", "tconstruct:ambush"]).to_bits(), raging_first.to_bits());
}

#[test]
fn raging_melee_multiplier_uses_global_and_emerald_not_base_stat_multipliers() {
 use horizon_items::items::ItemStack; use horizon_world::nbt::Tag; use std::collections::BTreeMap;
 let def=br#"{"modules":[{"type":"tconstruct:base_stats","stats":{"tconstruct:attack_damage":4.0}},{"type":"tconstruct:multiply_stats","multipliers":{"tconstruct:attack_damage":2.0}}]}"#;
 let mut stats=tconstruct_source_station_stats::base_stats(def,&[]).unwrap();
 // Handle percentages and Solid/Netherite modify the base stat, not the
 // separate conditional multiplier read by the source melee formula.
 stats.attack_damage_percent_raw=0.3;
 let mut tool=ItemStack::new("tconstruct:pickaxe",1);
 tool.data_mut().set_other("minecraft:custom_data",Some(Tag::compound([
  ("tic_upgrades",Tag::compound([("tconstruct:emerald",Tag::Int(1)),("tconstruct:netherite",Tag::Int(1))]))
 ])));
 let traits=BTreeMap::from([("tconstruct:raging".into(),1),("tconstruct:solid".into(),2),("tconstruct:ductile".into(),1)]);
 let multiplier=|tool:&ItemStack| match tool.data().unwrap().other("minecraft:custom_data").unwrap().get("tic_raging_melee_multiplier") {Some(Tag::Float(bits))=>f32::from_bits(*bits),_=>panic!("missing multiplier")};
 tconstruct_source_upgrade_rules::apply_stats(&mut tool,&stats,&traits,true,true).unwrap();
 assert_eq!(multiplier(&tool),2.5);
 tconstruct_source_upgrade_rules::apply_stats(&mut tool,&stats,&traits,true,true).unwrap();
 assert_eq!(multiplier(&tool),2.5);
 let mut custom=tool.data().unwrap().other("minecraft:custom_data").unwrap().clone();
 let Tag::Compound(ref mut fields)=custom else { panic!("expected compound") };
 fields.insert("tic_upgrades".into(),Tag::compound([("tconstruct:netherite",Tag::Int(1))]));
 tool.data_mut().set_other("minecraft:custom_data",Some(custom));
 tconstruct_source_upgrade_rules::apply_stats(&mut tool,&stats,&traits,true,true).unwrap();
 assert_eq!(multiplier(&tool),2.0);
 let mut trait_emerald=traits.clone(); trait_emerald.insert("tconstruct:emerald".into(),1);
 tconstruct_source_upgrade_rules::apply_stats(&mut tool,&stats,&trait_emerald,true,true).unwrap();
 assert_eq!(multiplier(&tool),2.5);
}

#[test]
fn airborne_overwield_use_conditional_not_base_multipliers() {
 use horizon_items::items::ItemStack; use horizon_world::nbt::Tag; use std::collections::BTreeMap;
 let def=br#"{"modules":[{"type":"tconstruct:base_stats","stats":{"tconstruct:draw_speed":0.25,"tconstruct:velocity":1.0,"tconstruct:accuracy":1.0}},{"type":"tconstruct:multiply_stats","multipliers":{"tconstruct:draw_speed":2.0,"tconstruct:velocity":3.0}}]}"#;
 let mut stats=tconstruct_source_station_stats::base_stats(def,&[]).unwrap();
 stats.draw_speed_raw[2]=0.5; stats.velocity_raw[2]=0.5;
 for id in ["tconstruct:longbow","tconstruct:crossbow"] {
  let mut bow=ItemStack::new(id,1);
  bow.data_mut().set_other("minecraft:custom_data",Some(Tag::compound([
   ("tic_max_damage",Tag::Int(100)),("tic_overslime",Tag::Int(1)),
   ("tic_upgrades",Tag::compound([("tconstruct:netherite",Tag::Int(1)),("tconstruct:quick_charge",Tag::Int(5))]))
  ])));
  let levels=BTreeMap::from([("tconstruct:airborne".into(),1),("tconstruct:overwield".into(),1),("tconstruct:vintage".into(),1)]);
  tconstruct_source_upgrade_rules::apply_stats(&mut bow,&stats,&levels,false,false).unwrap();
  let custom=bow.data().unwrap().other("minecraft:custom_data").unwrap();
  let value=|key| match custom.get(key) {Some(Tag::Float(bits))=>f32::from_bits(*bits),_=>panic!("missing {key}")};
  assert!((value("tic_airborne_velocity_multiplier")-3.0).abs()<1e-6);
  assert!((value("tic_overwield_velocity_multiplier")-3.0).abs()<1e-6);
  assert!((value("tic_overwield_draw_multiplier")-1.8).abs()<1e-6);
  assert_eq!(draw::draw_ticks(&bow),22);
  let base=value("tic_velocity");
  assert!((projectile::projectile_stats_at_airborne(&bow,64.0,320,true).unwrap().0-base-0.48).abs()<1e-5);
  tconstruct_source_upgrade_rules::apply_stats(&mut bow,&stats,&levels,false,false).unwrap();
  let custom=bow.data().unwrap().other("minecraft:custom_data").unwrap();
  assert!(matches!(custom.get("tic_overwield_draw_multiplier"),Some(Tag::Float(bits)) if (f32::from_bits(*bits)-1.8).abs()<1e-6));
  let without_vintage=BTreeMap::from([("tconstruct:airborne".into(),1),("tconstruct:overwield".into(),1)]);
  tconstruct_source_upgrade_rules::apply_stats(&mut bow,&stats,&without_vintage,false,false).unwrap();
  let custom=bow.data().unwrap().other("minecraft:custom_data").unwrap();
  assert!(matches!(custom.get("tic_overwield_draw_multiplier"),Some(Tag::Float(bits)) if (f32::from_bits(*bits)-2.0).abs()<1e-6));
 }
}

#[test]
fn raging_and_ambush_projectile_exclude_base_percent_from_conditional_multiplier() {
 use horizon_items::items::ItemStack; use horizon_world::nbt::Tag; use std::collections::BTreeMap;
 let def=br#"{"modules":[{"type":"tconstruct:base_stats","stats":{"tconstruct:projectile_damage":2.0}},{"type":"tconstruct:multiply_stats","multipliers":{"tconstruct:projectile_damage":2.0}}]}"#;
 let mut stats=tconstruct_source_station_stats::base_stats(def,&[]).unwrap();
 stats.projectile_damage_raw[2]=0.5;
 for id in ["tconstruct:longbow","tconstruct:crossbow"] {
  let mut bow=ItemStack::new(id,1);
  bow.data_mut().set_other("minecraft:custom_data",Some(Tag::compound(Vec::<(&str,Tag)>::new())));
  let traits=BTreeMap::from([("tconstruct:raging".into(),2),("tconstruct:ambush".into(),2)]);
  for _ in 0..2 {
   tconstruct_source_upgrade_rules::apply_stats(&mut bow,&stats,&traits,false,false).unwrap();
   let base=projectile::projectile_bonus(&bow);
   assert_eq!(base,4.0); // 2 * 1.5 * 2, minus the host's base 2.
   assert!((projectile::projectile_bonus_at(&bow,Some(6.0),Some(20.0))-base-2.0).abs()<1e-6);
   assert!((projectile::projectile_bonus_at(&bow,Some(20.0),Some(20.0))-base-2.0).abs()<1e-6);
   assert!((projectile::projectile_bonus_at(&bow,Some(6.0),Some(6.0))-base-4.0).abs()<1e-6);
  }
  tconstruct_source_upgrade_rules::apply_stats(&mut bow,&stats,&BTreeMap::new(),false,false).unwrap();
  assert_eq!(projectile::projectile_bonus_at(&bow,Some(6.0),Some(6.0)),projectile::projectile_bonus(&bow));
  let mut changed_stats=stats; changed_stats.projectile_damage_raw[3]=3.0;
  tconstruct_source_upgrade_rules::apply_stats(&mut bow,&changed_stats,&traits,false,false).unwrap();
  let base=projectile::projectile_bonus(&bow);
  assert!((projectile::projectile_bonus_at(&bow,Some(6.0),Some(6.0))-base-6.0).abs()<1e-6);
 }
}

#[test]
fn dwarven_velocity_depth_bonus_uses_conditional_multiplier() {
 use horizon_items::items::ItemStack; use horizon_world::nbt::Tag; use std::collections::BTreeMap;
 let def=br#"{"modules":[{"type":"tconstruct:base_stats","stats":{"tconstruct:velocity":1.0,"tconstruct:accuracy":1.0}},{"type":"tconstruct:multiply_stats","multipliers":{"tconstruct:velocity":3.0}}]}"#;
 let mut stats=tconstruct_source_station_stats::base_stats(def,&[]).unwrap();
 stats.velocity_raw[2]=0.5;
 for id in ["tconstruct:longbow","tconstruct:crossbow"] {
  let mut bow=ItemStack::new(id,1);
  bow.data_mut().set_other("minecraft:custom_data",Some(Tag::compound([
   ("tic_upgrades",Tag::compound([("tconstruct:netherite",Tag::Int(1))]))
  ])));
  let traits=BTreeMap::from([("tconstruct:dwarven".into(),2)]);
  for _ in 0..2 {
   tconstruct_source_upgrade_rules::apply_stats(&mut bow,&stats,&traits,false,false).unwrap();
   let base=projectile::projectile_stats(&bow).unwrap().0;
   assert!((base-4.95).abs()<1e-5);
   assert!((projectile::projectile_stats_at(&bow,0.0,320).unwrap().0-base-0.3).abs()<1e-5);
   assert!((projectile::projectile_stats_at(&bow,256.0,320).unwrap().0-base*0.25).abs()<1e-5);
  }
  tconstruct_source_upgrade_rules::apply_stats(&mut bow,&stats,&BTreeMap::new(),false,false).unwrap();
  assert_eq!(projectile::projectile_stats_at(&bow,0.0,320).unwrap().0,projectile::projectile_stats(&bow).unwrap().0);
 }
}

#[test]
fn diamond_trait_adds_melee_and_projectile_stats_once() {
 use horizon_items::items::ItemStack; use horizon_world::nbt::Tag; use std::collections::BTreeMap;
 let def=br#"{"modules":[{"type":"tconstruct:base_stats","stats":{"tconstruct:attack_damage":4.0,"tconstruct:projectile_damage":2.0}},{"type":"tconstruct:multiply_stats","multipliers":{"tconstruct:attack_damage":2.0,"tconstruct:projectile_damage":3.0}}]}"#;
 let stats=tconstruct_source_station_stats::base_stats(def,&[]).unwrap();
 for id in ["tconstruct:longbow","tconstruct:crossbow"] {
  let mut tool=ItemStack::new(id,1);
  tool.data_mut().set_other("minecraft:custom_data",Some(Tag::compound(Vec::<(&str,Tag)>::new())));
  let traits=BTreeMap::from([("tconstruct:diamond".into(),2)]);
  let read=|tool:&ItemStack,key| match tool.data().unwrap().other("minecraft:custom_data").unwrap().get(key){Some(Tag::Float(v))=>f32::from_bits(*v),_=>panic!("missing {key}")};
  for _ in 0..2 {
   tconstruct_source_upgrade_rules::apply_stats(&mut tool,&stats,&traits,true,true).unwrap();
   assert_eq!(read(&tool,"tic_attack_damage"),9.0);
   assert_eq!(read(&tool,"tic_projectile_damage"),7.5);
  }
  let mut custom=tool.data().unwrap().other("minecraft:custom_data").unwrap().clone();
  let Tag::Compound(ref mut fields)=custom else {panic!()};
  fields.insert("tic_upgrades".into(),Tag::compound([("tconstruct:diamond",Tag::Int(1))]));
  tool.data_mut().set_other("minecraft:custom_data",Some(custom));
  tconstruct_source_upgrade_rules::apply_stats(&mut tool,&stats,&traits,true,true).unwrap();
  assert_eq!(read(&tool,"tic_attack_damage"),9.0); assert_eq!(read(&tool,"tic_projectile_damage"),7.5);
  let mut custom=tool.data().unwrap().other("minecraft:custom_data").unwrap().clone();
  let Tag::Compound(ref mut fields)=custom else {panic!()}; fields.remove(&"tic_upgrades".into());
  tool.data_mut().set_other("minecraft:custom_data",Some(custom));
  tconstruct_source_upgrade_rules::apply_stats(&mut tool,&stats,&BTreeMap::new(),true,true).unwrap();
  assert_eq!(read(&tool,"tic_attack_damage"),8.0); assert_eq!(read(&tool,"tic_projectile_damage"),6.0);
 }
}

#[test]
fn diamond_trait_does_not_write_melee_stats_without_melee_tag() {
 use horizon_items::items::ItemStack; use horizon_world::nbt::Tag; use std::collections::BTreeMap;
 let def=br#"{"modules":[{"type":"tconstruct:base_stats","stats":{"tconstruct:attack_damage":4.0}}]}"#;
 let stats=tconstruct_source_station_stats::base_stats(def,&[]).unwrap(); let mut tool=ItemStack::new("tconstruct:pickaxe",1);
 tool.data_mut().set_other("minecraft:custom_data",Some(Tag::compound(Vec::<(&str,Tag)>::new())));
 tconstruct_source_upgrade_rules::apply_stats(&mut tool,&stats,&BTreeMap::from([("tconstruct:diamond".into(),1)]),false,false).unwrap();
 assert!(tool.data().unwrap().other("minecraft:custom_data").unwrap().get("tic_attack_damage").is_none());
}

#[test]
fn gem_traits_apply_durability_and_armor_once() {
 use horizon_items::items::ItemStack; use horizon_world::nbt::Tag; use std::collections::BTreeMap;
 let def=br#"{"modules":[{"type":"tconstruct:base_stats","stats":{"tconstruct:durability":100.0,"tconstruct:armor":2.0,"tconstruct:armor_toughness":1.0}}]}"#;
 let stats=tconstruct_source_station_stats::base_stats(def,&[]).unwrap();
 let traits=BTreeMap::from([("tconstruct:diamond".into(),2),("tconstruct:emerald".into(),2)]);
 for armor in [false,true] {
  let mut tool=ItemStack::new(if armor {"tconstruct:plate_chestplate"} else {"tconstruct:pickaxe"},1);
  tool.data_mut().set_other("minecraft:custom_data",Some(Tag::compound(Vec::<(&str,Tag)>::new())));
  for _ in 0..2 {
   tconstruct_source_upgrade_rules::apply_gem_durability_and_armor(&mut tool,&stats,&traits,armor,armor).unwrap();
   let c=tool.data().unwrap().other("minecraft:custom_data").unwrap();
   assert!(matches!(c.get("tic_max_damage"),Some(Tag::Int(n)) if *n == if armor {525} else {900}));
   assert!(matches!(tool.data().unwrap().other("minecraft:max_damage"),Some(Tag::Int(n)) if *n == if armor {525} else {900}));
   if armor {
    assert!(matches!(c.get("tic_armor"),Some(Tag::Float(n)) if f32::from_bits(*n)==3.0));
    assert!(matches!(c.get("tic_armor_toughness"),Some(Tag::Float(n)) if f32::from_bits(*n)==2.0));
    assert!(matches!(c.get("tconstruct:diamond_armor"),Some(Tag::Byte(1))));
   }
  }
  let mut custom=tool.data().unwrap().other("minecraft:custom_data").unwrap().clone(); let Tag::Compound(ref mut fields)=custom else {panic!()};
  fields.insert("tic_upgrades".into(),Tag::compound([("tconstruct:diamond",Tag::Int(1)),("tconstruct:emerald",Tag::Int(1))]));
  tool.data_mut().set_other("minecraft:custom_data",Some(custom));
  tconstruct_source_upgrade_rules::apply_gem_durability_and_armor(&mut tool,&stats,&traits,armor,armor).unwrap();
  assert!(matches!(tool.data().unwrap().other("minecraft:max_damage"),Some(Tag::Int(n)) if *n == if armor {525} else {900}));
  if armor {
   tconstruct_source_upgrade_rules::apply_gem_durability_and_armor(&mut tool,&stats,&traits,true,false).unwrap();
   assert!(tool.data().unwrap().other("minecraft:custom_data").unwrap().get("tconstruct:diamond_armor").is_none());
  }
 }
}

#[test]
fn gem_trait_removal_after_base_reset_clears_armor_flags() {
 use horizon_items::items::ItemStack; use horizon_world::nbt::Tag; use std::collections::BTreeMap;
 let def=br#"{"modules":[{"type":"tconstruct:base_stats","stats":{"tconstruct:durability":100.0,"tconstruct:armor":2.0,"tconstruct:armor_toughness":1.0}}]}"#;
 let stats=tconstruct_source_station_stats::base_stats(def,&[]).unwrap(); let mut tool=ItemStack::new("tconstruct:plate_chestplate",1);
 tool.data_mut().set_other("minecraft:custom_data",Some(Tag::compound(Vec::<(&str,Tag)>::new())));
 let emerald=BTreeMap::from([("tconstruct:emerald".into(),1)]);
 tconstruct_source_upgrade_rules::apply_gem_durability_and_armor(&mut tool,&stats,&emerald,true,true).unwrap();
 assert!(matches!(tool.data().unwrap().other("minecraft:max_damage"),Some(Tag::Int(150))));
 let both=BTreeMap::from([("tconstruct:diamond".into(),1),("tconstruct:emerald".into(),1)]);
 tconstruct_source_upgrade_rules::apply_gem_durability_and_armor(&mut tool,&stats,&both,true,true).unwrap();
 // The assembly path first resets these fields from the base definition.
 let mut custom=tool.data().unwrap().other("minecraft:custom_data").unwrap().clone();
 let Tag::Compound(ref mut fields)=custom else {panic!()}; fields.insert("tic_max_damage".into(),Tag::Int(100));
 tool.data_mut().set_other("minecraft:custom_data",Some(custom));
 tool.data_mut().set_other("minecraft:max_damage",Some(Tag::Int(100)));
 tconstruct_source_upgrade_rules::apply_gem_durability_and_armor(&mut tool,&stats,&BTreeMap::new(),true,true).unwrap();
 let c=tool.data().unwrap().other("minecraft:custom_data").unwrap();
 assert!(matches!(c.get("tic_max_damage"),Some(Tag::Int(100))));
 assert!(matches!(tool.data().unwrap().other("minecraft:max_damage"),Some(Tag::Int(100))));
 assert!(matches!(c.get("tic_armor"),Some(Tag::Float(v)) if f32::from_bits(*v)==2.0));
 assert!(matches!(c.get("tic_armor_toughness"),Some(Tag::Float(v)) if f32::from_bits(*v)==1.0));
 assert!(c.get("tconstruct:diamond_armor").is_none()); assert!(c.get("tic_attack_conditional_multiplier").is_none());
}

#[test]
fn resolved_gem_traits_set_harvest_tier_floors_without_lowering() {
 use std::collections::BTreeMap;
 let choose=|base,upgrades,traits| tconstruct_source_station_stats::gem_tier_with_traits(base,upgrades,traits,true);
 let diamond=BTreeMap::from([("tconstruct:diamond".into(),1)]);
 let emerald=BTreeMap::from([("tconstruct:emerald".into(),1)]);
 let netherite=BTreeMap::from([("tconstruct:netherite".into(),1)]);
 let none=[false,false,false]; let empty=BTreeMap::new();
 assert_eq!(choose(Some("stone"),none,Some(&emerald)),Some("iron"));
 assert_eq!(choose(Some("stone"),none,Some(&diamond)),Some("diamond"));
 assert_eq!(choose(Some("diamond"),none,Some(&emerald)),Some("diamond"));
 assert_eq!(choose(Some("stone"),none,Some(&netherite)),Some("netherite"));
 assert_eq!(choose(Some("stone"),[true,false,false],Some(&diamond)),Some("diamond"));
 assert_eq!(choose(Some("stone"),none,Some(&empty)),Some("stone"));
 assert_eq!(choose(Some("stone"),[false,true,false],None),Some("iron"));
 assert_eq!(choose(None,none,Some(&netherite)),None);
 assert_eq!(tconstruct_source_station_stats::gem_tier_with_traits(Some("stone"),none,Some(&diamond),false),Some("stone"));
}

#[test]
fn emerald_repair_factor_follows_source_modifier_order() {
 use horizon_items::items::ItemStack; use horizon_world::nbt::Tag; use std::collections::BTreeMap;
 let levels=BTreeMap::from([("tconstruct:emerald".into(),1),("tconstruct:cultivated".into(),1),("tconstruct:preserved".into(),2),("tconstruct:dense".into(),3)]);
 let run=|order:&[&str],upgrade:bool| {
  let mut tool=ItemStack::new("tconstruct:pickaxe",1);
  let upgrades=if upgrade {Tag::compound([("tconstruct:emerald",Tag::Int(1))])} else {Tag::compound(Vec::<(&str,Tag)>::new())};
  tool.data_mut().set_other("minecraft:custom_data",Some(Tag::compound([
   ("tic_upgrades",upgrades), ("tic_trait_order",Tag::List(order.iter().map(|id|Tag::String((*id).into())).collect()))
  ])));
  tconstruct_source_material_effects::ordered_known_repair_amount(&tool,&levels,&order.iter().map(|id|(*id).to_owned()).collect::<Vec<_>>(),1.0).unwrap()
 };
 let ids=["tconstruct:emerald","tconstruct:cultivated","tconstruct:preserved","tconstruct:dense"];
 let reverse=["tconstruct:dense","tconstruct:preserved","tconstruct:cultivated","tconstruct:emerald"];
 let first=(((1.0f32*1.5)*1.5)*1.3)*(1.0-0.025*3.0*8.0);
 let last=(((1.0f32*(1.0-0.025*3.0*8.0))*1.3)*1.5)*1.5;
 assert_ne!(first.to_bits(),last.to_bits());
 assert_eq!(run(&ids,false).to_bits(),first.to_bits());
 assert_eq!(run(&reverse,false).to_bits(),last.to_bits());
 assert_eq!(run(&ids,true).to_bits(),first.to_bits());
 // Dense >=6 uses the source fallback's 1 + (flat + per-level),
 // which can round differently from algebraically equivalent subtraction.
 let mut tool=ItemStack::new("tconstruct:pickaxe",1);
 tool.data_mut().set_other("minecraft:custom_data",Some(Tag::compound(Vec::<(&str,Tag)>::new())));
 for level in 7..=9 {
  let high=BTreeMap::from([("tconstruct:emerald".into(),1),("tconstruct:dense".into(),level)]);
  let expected=(1.0f32*(1.0+(-0.5 + -0.05*level as f32)))*1.5;
  assert_eq!(tconstruct_source_material_effects::ordered_known_repair_amount(&tool,&high,&["tconstruct:dense".into(),"tconstruct:emerald".into()],1.0).unwrap().to_bits(),expected.to_bits());
 }
 // As the Station does, replace a stale saved order with resolved order.
 tconstruct_source_material_effects::store_trait_order(&mut tool,&reverse.iter().map(|id|(*id).to_owned()).collect::<Vec<_>>()).unwrap();
 tconstruct_source_material_effects::store_trait_order(&mut tool,&ids.iter().map(|id|(*id).to_owned()).collect::<Vec<_>>()).unwrap();
 assert_eq!(tconstruct_source_material_effects::ordered_known_repair_amount(&tool,&levels,&ids.iter().map(|id|(*id).to_owned()).collect::<Vec<_>>(),1.0).unwrap().to_bits(),first.to_bits());
}

#[test]
fn emerald_material_repair_uses_per_item_amount_and_rejects_ambiguous_order() {
 use horizon_items::items::ItemStack; use horizon_world::nbt::Tag; use std::collections::BTreeMap;
 let traits=BTreeMap::from([("tconstruct:emerald".into(),1),("tconstruct:dense".into(),7)]);
 let order=vec!["tconstruct:emerald".into(),"tconstruct:dense".into()];
 let mut tool=ItemStack::new("tconstruct:pickaxe",1);
 tool.data_mut().set_other("minecraft:custom_data",Some(Tag::compound([
  ("tic_upgrades",Tag::compound([("tconstruct:dense",Tag::Int(0))])),
  ("tic_upgrade_order",Tag::List(vec![Tag::String("tconstruct:dense".into())]))
 ])));
 // A stale zero-level upgrade must not move Dense ahead of fresh trait order.
 let actual=tconstruct_source_material_effects::ordered_known_repair_amount(&tool,&traits,&order,100.0).unwrap();
 let expected=(100.0f32*1.5)*(1.0+(-0.5 + -0.05*7.0));
 assert_eq!(actual.to_bits(),expected.to_bits());
 // Source ceil(needed / per-item) crosses the two-to-three-item boundary.
 assert_eq!((45.0f32 / actual).ceil() as u32,3);
 let ambiguous=Tag::compound([
  ("tic_upgrades",Tag::compound([("tconstruct:emerald",Tag::Int(1)),("tconstruct:preserved",Tag::Int(1))]))
 ]);
 tool.data_mut().set_other("minecraft:custom_data",Some(ambiguous));
 assert!(tconstruct_source_material_effects::ordered_known_repair_amount(&tool,&traits,&order,100.0).is_none());
 let missing_trait_order=vec!["tconstruct:emerald".into()];
 tool.data_mut().set_other("minecraft:custom_data",Some(Tag::compound(Vec::<(&str,Tag)>::new())));
 assert!(tconstruct_source_material_effects::ordered_known_repair_amount(&tool,&traits,&missing_trait_order,100.0).is_none());
}

#[test]
fn emerald_modifier_repair_uses_factor_then_truncates_per_item() {
 use horizon_items::items::ItemStack; use horizon_world::nbt::Tag; use std::collections::BTreeMap;
 let mut tool=ItemStack::new("tconstruct:pickaxe",1);
 tool.data_mut().set_other("minecraft:custom_data",Some(Tag::compound([("tic_upgrades",Tag::compound([("tconstruct:emerald",Tag::Int(1))]))])));
 let traits=BTreeMap::from([("tconstruct:emerald".into(),1),("tconstruct:preserved".into(),1)]);
 let order=vec!["tconstruct:preserved".into(),"tconstruct:emerald".into()];
 let factor=tconstruct_source_material_effects::ordered_known_repair_amount(&tool,&traits,&order,1.0).unwrap();
 assert_eq!(factor.to_bits(),(1.5f32*1.15).to_bits());
 let per_item=(7.0f32*factor) as u32;
 assert_eq!(per_item,12);
 let needed=25u32; let consumed=needed.div_ceil(per_item); let repaired=needed.min(consumed*per_item);
 assert_eq!((consumed,repaired),(3,25));
}

#[test]
fn emerald_casting_repair_trait_is_flat_and_mixed_hooks_remain_closed() {
 use std::collections::BTreeMap;
 let factor=tconstruct_source_material_effects::limited_casting_repair_factor;
 let emerald=BTreeMap::from([("tconstruct:emerald".into(),2)]);
 assert_eq!(factor(&emerald,false),Some(1.5));
 assert_eq!(factor(&emerald,true),Some(1.5));
 assert_eq!(factor(&BTreeMap::new(),true),Some(1.5));
 assert_eq!(factor(&BTreeMap::new(),false),Some(1.0));
 let mixed=BTreeMap::from([("tconstruct:emerald".into(),1),("tconstruct:dense".into(),1)]);
 assert_eq!(factor(&mixed,false),None);
 let unknown=BTreeMap::from([("test:unknown".into(),1)]);
 assert_eq!(factor(&unknown,false),None);
 // Source first scales material durability by item-cost / three, then applies
 // the one permitted repair hook and truncates the result.
 let repair=(101.0f32*(2.0/3.0)*factor(&emerald,true).unwrap()) as u32;
 assert_eq!(repair,101);
}

#[test]
fn dense_repair_high_levels_keep_source_rounding_at_cast_boundary() {
 use std::collections::BTreeMap;
 for level in 6..=10 {
  let expected=(1.0f32+(-0.5 + -0.05*level as f32)).max(0.0);
  assert_eq!(tconstruct_source_material_effects::dense_repair_factor(level).to_bits(),expected.to_bits());
  let traits=BTreeMap::from([("tconstruct:dense".into(),level)]);
  assert_eq!(tconstruct_source_material_effects::limited_casting_repair_factor(&traits,false).unwrap().to_bits(),expected.to_bits());
 }
 assert_eq!(tconstruct_source_material_effects::dense_repair_factor(11),0.0);
 assert_eq!(tconstruct_source_material_effects::dense_repair_factor(i32::MAX),0.0);
 let seven=tconstruct_source_material_effects::limited_casting_repair_factor(&BTreeMap::from([("tconstruct:dense".into(),7)]),false).unwrap();
 let eight=tconstruct_source_material_effects::limited_casting_repair_factor(&BTreeMap::from([("tconstruct:dense".into(),8)]),false).unwrap();
 assert_eq!((20.0f32*seven) as u32,2);
 assert_eq!((10.0f32*eight) as u32,1);
}

#[test]
fn emerald_casting_mixed_traits_apply_to_rebuilt_tool_in_source_order() {
 use horizon_items::items::ItemStack; use horizon_world::nbt::Tag; use std::collections::BTreeMap;
 let mut tool=ItemStack::new("tconstruct:pickaxe",1);
 tool.data_mut().set_other("minecraft:custom_data",Some(Tag::compound([("tic_upgrades",Tag::compound([("tconstruct:emerald",Tag::Int(1))]))])));
 let traits=BTreeMap::from([("tconstruct:emerald".into(),1),("tconstruct:preserved".into(),2),("tconstruct:dense".into(),7)]);
 let order=vec!["tconstruct:dense".into(),"tconstruct:emerald".into(),"tconstruct:preserved".into()];
 let actual=tconstruct_source_material_effects::casting_repair_amount(&tool,&traits,&order,20.0).unwrap();
 // The installed Emerald runs first, then resolved Dense and Preserved.
 let expected=((20.0f32*1.5)*(1.0+(-0.5 + -0.05*7.0)))*1.3;
 assert_eq!(actual.to_bits(),expected.to_bits());
 assert_eq!(actual as u32,5);
 let unknown=BTreeMap::from([("tconstruct:emerald".into(),1),("test:unknown".into(),1)]);
 assert!(tconstruct_source_material_effects::casting_repair_amount(&tool,&unknown,&order,20.0).is_none());
 let without_emerald=BTreeMap::from([("tconstruct:dense".into(),1),("tconstruct:preserved".into(),1)]);
 let mixed_order=vec!["tconstruct:dense".into(),"tconstruct:preserved".into()];
 // This fixture still has Emerald installed; remove it for the no-gem path.
 tool.data_mut().set_other("minecraft:custom_data",Some(Tag::compound(Vec::<(&str,Tag)>::new())));
 let mixed=tconstruct_source_material_effects::casting_repair_amount(&tool,&without_emerald,&mixed_order,20.0).unwrap();
 assert_eq!(mixed.to_bits(),((20.0f32*(1.0-0.025*1.0*10.0))*1.15).to_bits());
 tool.data_mut().set_other("minecraft:custom_data",Some(Tag::compound(Vec::<(&str,Tag)>::new())));
 let trait_only=BTreeMap::from([("tconstruct:emerald".into(),1),("tconstruct:preserved".into(),1)]);
 let trait_order=vec!["tconstruct:preserved".into(),"tconstruct:emerald".into()];
 let expected=(20.0f32*1.15)*1.5;
 assert_eq!(tconstruct_source_material_effects::casting_repair_amount(&tool,&trait_only,&trait_order,20.0).unwrap().to_bits(),expected.to_bits());
 let zero=BTreeMap::from([("tconstruct:emerald".into(),1),("tconstruct:dense".into(),10)]);
 let zero_order=vec!["tconstruct:dense".into(),"tconstruct:emerald".into()];
 assert_eq!(tconstruct_source_material_effects::casting_repair_amount(&tool,&zero,&zero_order,20.0),Some(0.0));
}

#[test]
fn casting_known_mixed_repair_uses_fresh_order_and_gem_upgrade() {
 use horizon_items::items::ItemStack; use horizon_world::nbt::Tag; use std::collections::BTreeMap;
 let mut tool=ItemStack::new("tconstruct:pickaxe",1);
 tool.data_mut().set_other("minecraft:custom_data",Some(Tag::compound([
  ("tic_upgrades",Tag::compound([("tconstruct:emerald",Tag::Int(1))])),
  ("tic_trait_order",Tag::List(vec![Tag::String("tconstruct:preserved".into()),Tag::String("tconstruct:dense".into())]))
 ])));
 let traits=BTreeMap::from([("tconstruct:dense".into(),7),("tconstruct:preserved".into(),2)]);
 let order=vec!["tconstruct:dense".into(),"tconstruct:preserved".into()];
 let actual=tconstruct_source_material_effects::casting_repair_amount(&tool,&traits,&order,20.0).unwrap();
 let expected=((20.0f32*1.5)*(1.0+(-0.5 + -0.05*7.0)))*1.3;
 assert_eq!(actual.to_bits(),expected.to_bits());
 assert_eq!(actual as u32,5);
 let missing=vec!["tconstruct:dense".into()];
 assert!(tconstruct_source_material_effects::casting_repair_amount(&tool,&traits,&missing,20.0).is_none());
 tool.data_mut().set_other("minecraft:custom_data",Some(Tag::compound([("tic_upgrades",Tag::compound([("tconstruct:emerald",Tag::String("bad".into()))]))])));
 assert!(tconstruct_source_material_effects::casting_repair_amount(&tool,&traits,&order,20.0).is_none());
 assert!(tconstruct_source_material_effects::casting_repair_amount(&tool,&traits,&order,f32::NAN).is_none());
}

#[test]
fn material_repair_without_emerald_uses_per_item_source_order() {
 use horizon_items::items::ItemStack; use horizon_world::nbt::Tag; use std::collections::BTreeMap;
 let mut tool=ItemStack::new("tconstruct:pickaxe",1);
 tool.data_mut().set_other("minecraft:custom_data",Some(Tag::compound(Vec::<(&str,Tag)>::new())));
 let traits=BTreeMap::from([("tconstruct:preserved".into(),2),("tconstruct:dense".into(),7)]);
 let dense_first=vec!["tconstruct:dense".into(),"tconstruct:preserved".into()];
 let preserved_first=vec!["tconstruct:preserved".into(),"tconstruct:dense".into()];
 let initial=20.0f32;
 let a=tconstruct_source_material_effects::ordered_known_repair_amount(&tool,&traits,&dense_first,initial).unwrap();
 let b=tconstruct_source_material_effects::ordered_known_repair_amount(&tool,&traits,&preserved_first,initial).unwrap();
 assert_ne!(a.to_bits(),b.to_bits());
 assert_eq!(a.to_bits(),((initial*(1.0+(-0.5 + -0.05*7.0)))*1.3).to_bits());
 assert_eq!(b.to_bits(),((initial*1.3)*(1.0+(-0.5 + -0.05*7.0))).to_bits());
 let empty=BTreeMap::new();
 assert_eq!(tconstruct_source_material_effects::ordered_known_repair_amount(&tool,&empty,&[],20.0),Some(20.0));
 tool.data_mut().set_other("minecraft:custom_data",Some(Tag::compound([("tic_upgrades",Tag::compound([("tconstruct:emerald",Tag::Int(1))]))])));
 assert_eq!(tconstruct_source_material_effects::ordered_known_repair_amount(&tool,&empty,&[],20.0),Some(30.0));
}

#[test]
fn non_emerald_modifier_repair_factor_uses_resolved_order() {
 use horizon_items::items::ItemStack; use horizon_world::nbt::Tag; use std::collections::BTreeMap;
 let mut tool=ItemStack::new("tconstruct:pickaxe",1);
 tool.data_mut().set_other("minecraft:custom_data",Some(Tag::compound(Vec::<(&str,Tag)>::new())));
 let traits=BTreeMap::from([("tconstruct:dense".into(),7),("tconstruct:preserved".into(),2)]);
 let order=vec!["tconstruct:dense".into(),"tconstruct:preserved".into()];
 let factor=tconstruct_source_material_effects::ordered_known_repair_amount(&tool,&traits,&order,1.0).unwrap();
 let expected=(1.0f32*(1.0+(-0.5 + -0.05*7.0)))*1.3;
 assert_eq!(factor.to_bits(),expected.to_bits());
 assert_eq!((100.0f32*factor) as u32,19);
 let missing=vec!["tconstruct:dense".into()];
 assert!(tconstruct_source_material_effects::ordered_known_repair_amount(&tool,&traits,&missing,1.0).is_none());
 tool.data_mut().set_other("minecraft:custom_data",Some(Tag::compound([("tic_upgrades",Tag::compound([("tconstruct:preserved",Tag::Int(1))]))])));
 let combined=BTreeMap::from([("tconstruct:preserved".into(),1),("tconstruct:dense".into(),7)]);
 let actual=tconstruct_source_material_effects::ordered_known_repair_amount(&tool,&combined,&order,1.0).unwrap();
 assert_eq!(actual.to_bits(),((1.0f32*1.3)*(1.0+(-0.5 + -0.05*7.0))).to_bits());
 tool.data_mut().set_other("minecraft:custom_data",Some(Tag::compound([
  ("tic_upgrades",Tag::compound([("tconstruct:preserved",Tag::Int(1)),("tconstruct:cultivated",Tag::Int(1))])),
  ("tic_upgrade_order",Tag::List(vec![Tag::String("tconstruct:cultivated".into()),Tag::String("tconstruct:preserved".into())]))
 ])));
 let actual=tconstruct_source_material_effects::ordered_known_repair_amount(&tool,&combined,&order,1.0).unwrap();
 assert_eq!(actual.to_bits(),(((1.0f32*1.5)*1.3)*(1.0+(-0.5 + -0.05*7.0))).to_bits());
}

#[test]
fn casting_repair_uses_ordered_non_emerald_upgrades() {
 use horizon_items::items::ItemStack; use horizon_world::nbt::Tag; use std::collections::BTreeMap;
 let mut tool=ItemStack::new("tconstruct:pickaxe",1);
 tool.data_mut().set_other("minecraft:custom_data",Some(Tag::compound([
  ("tic_upgrades",Tag::compound([("tconstruct:preserved",Tag::Int(2)),("tconstruct:dense",Tag::Int(7))])),
  ("tic_upgrade_order",Tag::List(vec![Tag::String("tconstruct:dense".into()),Tag::String("tconstruct:preserved".into())]))
 ])));
 let traits=BTreeMap::from([("tconstruct:cultivated".into(),1)]);
 let order=vec!["tconstruct:cultivated".into()];
 let result=tconstruct_source_material_effects::casting_repair_amount(&tool,&traits,&order,20.0).unwrap();
 let expected=((20.0f32*(1.0+(-0.5 + -0.05*7.0)))*1.3)*1.5;
 assert_eq!(result.to_bits(),expected.to_bits());
 tool.data_mut().set_other("minecraft:custom_data",Some(Tag::compound([
  ("tic_upgrades",Tag::compound([("tconstruct:preserved",Tag::Int(2)),("tconstruct:dense",Tag::Int(7))]))
 ])));
 assert!(tconstruct_source_material_effects::casting_repair_amount(&tool,&traits,&order,20.0).is_none());
 tool.data_mut().set_other("minecraft:custom_data",Some(Tag::compound([
  ("tic_upgrades",Tag::compound([("tconstruct:cultivated",Tag::String("bad".into()))]))
 ])));
 assert!(tconstruct_source_material_effects::casting_repair_amount(&tool,&traits,&order,20.0).is_none());
}

#[test]
fn swiftstrike_regeneration_merges_logical_levels_before_float_scaling() {
 use horizon_items::items::ItemStack;use horizon_world::nbt::Tag;use std::collections::BTreeMap;
 let def=br#"{"modules":[{"type":"tconstruct:base_stats","stats":{"tconstruct:attack_damage":4,"tconstruct:attack_speed":1}}]}"#;
 let stats=tconstruct_source_station_stats::base_stats(def,&[]).unwrap();
 for (trait_level,progress) in [(0,84),(1,12),(2,75),(3,150),(0,0),(2,0),(2,71),(2,72),(2,73),(2,143),(2,144),(0,-1),(-1,12),(0,i32::MAX),(i32::MAX,1),(i32::MAX,72)] {
  let mut tool=ItemStack::new("tconstruct:sword",1);
  tool.data_mut().set_other("minecraft:custom_data",Some(Tag::compound([
   ("tic_upgrades",Tag::compound([("tconstruct:swiftstrike",Tag::Int(progress))])),
   ("fixture:foreign",Tag::String("keep".into())),
  ])));
  let levels=BTreeMap::from([("tconstruct:swiftstrike".into(),trait_level)]);
  let amount=progress.max(0);let remainder=amount%72;
  let logical=trait_level.max(0).checked_add(amount/72).and_then(|n|n.checked_add(i32::from(remainder!=0)));
  let effective=logical.map(|n|if remainder==0 {n as f32}else{n as f32+(remainder-72) as f32/72.0}).unwrap_or(0.0);
  let expected=(1.0f32+0.05f32*effective).clamp(0.0,1024.0);
  for _ in 0..2 {
   tconstruct_source_upgrade_rules::apply_stats(&mut tool,&stats,&levels,true,true).unwrap();
   let custom=tool.data().unwrap().other("minecraft:custom_data").unwrap();
   assert_eq!(custom.get("tic_attack_speed"),Some(&Tag::Float(expected.to_bits())));
   assert_eq!(custom.get("fixture:foreign"),Some(&Tag::String("keep".into())));
   assert_eq!(custom.get("tic_upgrades").and_then(|u|u.get("tconstruct:swiftstrike")),Some(&Tag::Int(progress)));
   assert_eq!(custom.get("tic_attack_damage"),Some(&Tag::Float(4.0f32.to_bits())));
  }
  if [(0,84),(1,12),(2,75),(3,150)].contains(&(trait_level,progress)) {
   let old=1.0f32+0.05f32*(progress as f32/72.0+trait_level as f32);assert_ne!(expected.to_bits(),old.to_bits());
  }
  tconstruct_source_upgrade_rules::apply_stats(&mut tool,&stats,&levels,true,false).unwrap();
  assert_eq!(tool.data().unwrap().other("minecraft:custom_data").unwrap().get("tic_attack_speed"),Some(&Tag::Float(expected.to_bits())));
  tool.data_mut().set_other("minecraft:custom_data",Some(Tag::compound(Vec::<(&str,Tag)>::new())));
  tconstruct_source_upgrade_rules::apply_stats(&mut tool,&stats,&BTreeMap::new(),true,true).unwrap();
  assert_eq!(tool.data().unwrap().other("minecraft:custom_data").unwrap().get("tic_attack_speed"),Some(&Tag::Float(1.0f32.to_bits())));
 }
}

#[test]
fn sharpness_regeneration_merges_logical_levels_before_float_scaling() {
 use horizon_items::items::ItemStack;use horizon_world::nbt::Tag;use std::collections::BTreeMap;
 let def=br#"{"modules":[{"type":"tconstruct:base_stats","stats":{"tconstruct:attack_damage":1,"tconstruct:attack_speed":1}}]}"#;
 let stats=tconstruct_source_station_stats::base_stats(def,&[]).unwrap();
 for (trait_level,progress) in [(0,2),(1,255),(2,219),(3,183),(0,0),(2,0),(2,35),(2,36),(2,37),(2,71),(2,72),(0,-1),(-1,12),(0,i32::MAX),(i32::MAX,1),(i32::MAX,36)] {
  let mut tool=ItemStack::new("tconstruct:sword",1);
  tool.data_mut().set_other("minecraft:custom_data",Some(Tag::compound([
   ("tic_upgrades",Tag::compound([("tconstruct:sharpness",Tag::Int(progress))])),
   ("fixture:foreign",Tag::String("keep".into())),
  ])));
  let levels=BTreeMap::from([("tconstruct:sharpness".into(),trait_level)]);
  let amount=progress.max(0);let remainder=amount%36;
  let logical=trait_level.max(0).checked_add(amount/36).and_then(|n|n.checked_add(i32::from(remainder!=0)));
  let effective=logical.map(|n|if remainder==0 {n as f32}else{n as f32+(remainder-36) as f32/36.0}).unwrap_or(0.0);
  let expected=(1.0f32+0.75f32*effective).clamp(0.0,2048.0);
  for _ in 0..2 {
   tconstruct_source_upgrade_rules::apply_stats(&mut tool,&stats,&levels,true,true).unwrap();
   let custom=tool.data().unwrap().other("minecraft:custom_data").unwrap();
   assert_eq!(custom.get("tic_attack_damage"),Some(&Tag::Float(expected.to_bits())));
   assert_eq!(custom.get("fixture:foreign"),Some(&Tag::String("keep".into())));
   assert_eq!(custom.get("tic_upgrades").and_then(|u|u.get("tconstruct:sharpness")),Some(&Tag::Int(progress)));
   assert_eq!(custom.get("tic_attack_speed"),Some(&Tag::Float(1.0f32.to_bits())));
  }
  if [(0,2),(1,255),(2,219),(3,183)].contains(&(trait_level,progress)) {
   let old=1.0f32+0.75f32*(progress as f32/36.0+trait_level as f32);assert_ne!(expected.to_bits(),old.to_bits());
  }
  tconstruct_source_upgrade_rules::apply_stats(&mut tool,&stats,&levels,false,true).unwrap();
  assert_eq!(tool.data().unwrap().other("minecraft:custom_data").unwrap().get("tic_attack_damage"),Some(&Tag::Float(expected.to_bits())));
  tool.data_mut().set_other("minecraft:custom_data",Some(Tag::compound(Vec::<(&str,Tag)>::new())));
  tconstruct_source_upgrade_rules::apply_stats(&mut tool,&stats,&BTreeMap::new(),true,true).unwrap();
  assert_eq!(tool.data().unwrap().other("minecraft:custom_data").unwrap().get("tic_attack_damage"),Some(&Tag::Float(1.0f32.to_bits())));
 }
}

#[test]
fn trueshot_regeneration_scales_merged_logical_levels_before_accuracy_addition() {
 use horizon_items::items::ItemStack;use horizon_world::nbt::Tag;use std::collections::BTreeMap;
 for base in [0.1f32,0.11,0.13,0.15,0.75] {
  let def=format!(r#"{{"modules":[{{"type":"tconstruct:base_stats","stats":{{"tconstruct:accuracy":{base},"tconstruct:draw_speed":1,"tconstruct:velocity":1}}}}]}}"#);
  let stats=tconstruct_source_station_stats::base_stats(def.as_bytes(),&[]).unwrap();
  for (trait_level,progress) in [(0,1),(0,4),(0,0),(2,0),(2,9),(2,10),(2,11),(2,19),(2,20),(0,-1),(-1,1),(0,i32::MAX),(i32::MAX,1),(i32::MAX,10)] {
   for item in ["tconstruct:longbow","tconstruct:crossbow"] {
    let mut tool=ItemStack::new(item,1);
    tool.data_mut().set_other("minecraft:custom_data",Some(Tag::compound([
     ("tic_upgrades",Tag::compound([("tconstruct:trueshot",Tag::Int(progress))])),
     ("fixture:foreign",Tag::String("keep".into())),
    ])));
    let levels=BTreeMap::from([("tconstruct:trueshot".into(),trait_level)]);
    let amount=progress.max(0);let remainder=amount%10;
    let logical=trait_level.max(0).checked_add(amount/10).and_then(|n|n.checked_add(i32::from(remainder!=0)));
    let effective=logical.map(|n|if remainder==0 {n as f32}else{n as f32+(remainder-10) as f32/10.0}).unwrap_or(0.0);
    let expected=(base+0.1f32*effective).clamp(0.1,1.0);
    for _ in 0..2 {
     tconstruct_source_upgrade_rules::apply_stats(&mut tool,&stats,&levels,false,false).unwrap();
     let custom=tool.data().unwrap().other("minecraft:custom_data").unwrap();
     assert_eq!(custom.get("tic_accuracy"),Some(&Tag::Float(expected.to_bits())));
     assert_eq!(custom.get("fixture:foreign"),Some(&Tag::String("keep".into())));
     assert_eq!(custom.get("tic_upgrades").and_then(|u|u.get("tconstruct:trueshot")),Some(&Tag::Int(progress)));
     assert_eq!(custom.get("tic_draw_speed"),Some(&Tag::Float(1.0f32.to_bits())));
     assert_eq!(custom.get("tic_velocity"),Some(&Tag::Float(1.0f32.to_bits())));
    }
    if trait_level==0 && (([0.1f32,0.11].contains(&base)&&progress==1)||([0.13f32,0.15].contains(&base)&&progress==4)) {
     let old=(base+0.1f32*(progress as f32/10.0)).clamp(0.1,1.0);assert_ne!(expected.to_bits(),old.to_bits());
    }
    tool.item="tconstruct:sword".into();
    tconstruct_source_upgrade_rules::apply_stats(&mut tool,&stats,&levels,false,false).unwrap();
    assert_eq!(tool.data().unwrap().other("minecraft:custom_data").unwrap().get("tic_accuracy"),Some(&Tag::Float(expected.to_bits())));
    tool.item=item.into();
    tool.data_mut().set_other("minecraft:custom_data",Some(Tag::compound(Vec::<(&str,Tag)>::new())));
    tconstruct_source_upgrade_rules::apply_stats(&mut tool,&stats,&BTreeMap::new(),false,false).unwrap();
    assert_eq!(tool.data().unwrap().other("minecraft:custom_data").unwrap().get("tic_accuracy"),Some(&Tag::Float(base.to_bits())));
   }
  }
 }
}
