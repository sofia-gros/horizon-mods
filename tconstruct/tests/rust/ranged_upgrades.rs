extern crate horizon_items as horizon_client;
#[path = "../../mixins/tconstruct/source-dwarven.rs"] mod tconstruct_source_dwarven;
#[path = "../../mixins/tconstruct/source-station-stats.rs"] mod tconstruct_source_station_stats;
#[path = "../../mixins/tconstruct/source-melee-stat-ops.rs"] mod tconstruct_source_melee_stat_ops;
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
   // Base 2 * netherite 1.1 + maintained .16 * (2 * 1.1).
   assert!((projectile::projectile_stats(&bow).unwrap().0-2.552).abs()<0.00001);
  }
  bow.damage=50;
  assert!((projectile::projectile_stats(&bow).unwrap().0-2.2).abs()<0.00001);
  tconstruct_source_upgrade_rules::apply_stats(&mut bow,&stats,&BTreeMap::new(),false,false).unwrap();
  bow.damage=0;
  assert!((projectile::projectile_bonus(&bow)-5.5).abs()<0.00001);
  assert!((projectile::projectile_stats(&bow).unwrap().0-2.2).abs()<0.00001);
 }
}
