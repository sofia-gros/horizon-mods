extern crate horizon_items as horizon_client;
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
