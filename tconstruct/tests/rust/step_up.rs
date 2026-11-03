extern crate horizon_items as horizon_client;
#[path = "../../mixins/tconstruct/source-station-stats.rs"] mod tconstruct_source_station_stats;
#[path = "../../mixins/tconstruct/source-melee-stat-ops.rs"] mod tconstruct_source_melee_stat_ops;
#[path = "../../mixins/tconstruct/source-upgrade-rules.rs"] mod tconstruct_source_upgrade_rules;
use horizon_items::items::ItemStack;
use horizon_world::nbt::Tag;
use std::collections::{BTreeMap,HashMap,HashSet};
fn ingredients() -> Vec<Option<ItemStack>> {
 ["minecraft:leather","minecraft:golden_carrot","minecraft:leather","minecraft:scaffolding","minecraft:scaffolding"].iter().map(|x|Some(ItemStack::new(x,1))).collect()
}
fn recipes() -> BTreeMap<String,Vec<u8>> {
 let recipe=br#"{"type":"tconstruct:modifier","allow_crystal":true,"inputs":[{"item":"minecraft:leather"},{"item":"minecraft:golden_carrot"},{"item":"minecraft:leather"},{"item":"minecraft:scaffolding"},{"item":"minecraft:scaffolding"}],"level":{"max":2},"result":"tconstruct:step_up","slots":{"upgrades":1},"tools":{"tag":"tconstruct:modifiable/armor/leggings"}}"#;
 [("test:step".to_string(),recipe.to_vec())].into_iter().collect()
}
fn tags() -> HashMap<String,HashSet<String>> {
 [("tconstruct:modifiable/armor/leggings".to_string(),["tconstruct:travelers_leggings".to_string()].into_iter().collect()),("tconstruct:modifiable".to_string(),["tconstruct:travelers_leggings".to_string()].into_iter().collect())].into_iter().collect()
}
fn tool(old:i32) -> ItemStack {
 let mut t=ItemStack::new("tconstruct:travelers_leggings",1);
 t.data_mut().set_other("minecraft:custom_data",Some(Tag::compound([("tic_upgrades",Tag::compound([("tconstruct:step_up",Tag::Int(old))]))])));t
}
#[test] fn levels_and_maximum() {
 let mut r=tconstruct_source_upgrade_rules::plan_with_traits(&recipes(),&tags(),&tool(0),&ingredients(),2,0,&BTreeMap::new()).unwrap();assert_eq!(r.amount,1);assert_eq!(r.consumed.len(),6);
 r=tconstruct_source_upgrade_rules::plan_with_traits(&recipes(),&tags(),&tool(1),&ingredients(),2,0,&BTreeMap::new()).unwrap();assert_eq!(r.amount,2);
 assert!(tconstruct_source_upgrade_rules::plan_with_traits(&recipes(),&tags(),&tool(2),&ingredients(),3,0,&BTreeMap::new()).is_none());
}
#[test] fn rejects_wrong_item_slots_or_tag() {
 let mut bad=ingredients();bad[0]=Some(ItemStack::new("minecraft:stone",1));
 assert!(tconstruct_source_upgrade_rules::plan_with_traits(&recipes(),&tags(),&tool(0),&bad,2,0,&BTreeMap::new()).is_none());
 assert!(tconstruct_source_upgrade_rules::plan_with_traits(&recipes(),&tags(),&tool(0),&ingredients(),0,0,&BTreeMap::new()).is_none());
 assert!(tconstruct_source_upgrade_rules::plan_with_traits(&recipes(),&HashMap::new(),&tool(0),&ingredients(),2,0,&BTreeMap::new()).is_none());
}
#[test] fn extraction_and_material_trait_are_independent() {
 let traits=BTreeMap::from([("tconstruct:step_up".to_string(),1)]);
 let result=tconstruct_source_upgrade_rules::plan_with_traits(&recipes(),&tags(),&tool(0),&ingredients(),2,0,&traits).unwrap();
 assert_eq!(result.amount,1);
 let inputs=vec![Some(ItemStack::new("tconstruct:sky_slime_crystal",1)),Some(ItemStack::new("minecraft:wet_sponge",1))];
 let result=tconstruct_source_upgrade_rules::extract_upgrade_with_traits(&tags(),&tool(2),&inputs,"tconstruct:step_up",&BTreeMap::new()).unwrap();
 assert_eq!(result.upgrade.amount,1);
}
