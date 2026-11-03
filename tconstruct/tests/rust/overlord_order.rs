extern crate horizon_items as horizon_client;
#[path="../../mixins/tconstruct/source-station-stats.rs"] mod tconstruct_source_station_stats;
#[path="../../mixins/tconstruct/source-material-effects.rs"] mod tconstruct_source_material_effects;
#[path="../../../pluto-lib/mixins/pluto_native_creative/stack-fluid.rs"] mod pluto_stack_fluid;
#[path="../../mixins/tconstruct/source-smeltery-tank.rs"] mod tconstruct_source_smeltery_tank;
#[path="../../mixins/tconstruct/source-melter-state.rs"] mod tconstruct_source_melter_state;
#[path="../../mixins/tconstruct/source-smeltery-fuel.rs"] mod tconstruct_source_smeltery_fuel;
#[path="../../mixins/tconstruct/source-fluid-container.rs"] mod tconstruct_source_fluid_container;
#[path="../../mixins/tconstruct/source-upgrade-rules.rs"] mod tconstruct_source_upgrade_rules;


#[path="../../mixins/tconstruct/source-overslime.rs"] mod tconstruct_source_overslime;
#[test]
fn overlord_rebuild_combines_real_durability_and_capacity() {
 use horizon_items::items::ItemStack; use horizon_world::nbt::Tag; use std::collections::BTreeMap;
 let def=br#"{"modules":[{"type":"tconstruct:base_stats","stats":{"tconstruct:durability":100}}]}"#;
 let stats=tconstruct_source_station_stats::base_stats(def,&[]).unwrap();
 let mut tool=ItemStack::new("tconstruct:pickaxe",1);
 tool.data_mut().set_other("minecraft:custom_data",Some(Tag::compound([("tic_max_damage",Tag::Int(100)),("tic_overslime",Tag::Int(99)),("tic_upgrades",Tag::compound([("tconstruct:overslime",Tag::Int(1))]))])));
 let levels=BTreeMap::from([("tconstruct:overlord".to_string(),1)]);
 tconstruct_source_material_effects::apply_stat_traits(&mut tool,&stats,&levels).unwrap();
 tconstruct_source_overslime::rebuild(&mut tool,&stats,&levels,false,false,false,false).unwrap();
 let data=tool.data().unwrap().other("minecraft:custom_data").unwrap();
 assert!(matches!(data.get("tic_max_damage"),Some(Tag::Int(85))));
 assert!(matches!(data.get("tic_overslime_capacity"),Some(Tag::Int(60))));
 assert!(matches!(data.get("tic_overslime"),Some(Tag::Int(60))));
}
