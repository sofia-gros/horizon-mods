// Appended to the existing real lookup/state fixture. World registry and tool
// transport are bounded host doubles; assembly, traits, stats, swapping and
// the entire ordinary casting candidate branch are production source.
#[path="__MIXINS__/source-station-stats.rs"] mod tconstruct_source_station_stats;
#[path="__MIXINS__/source-station-recipes.rs"] mod tconstruct_source_station_recipes;
#[path="__MIXINS__/source-material-effects.rs"] mod tconstruct_source_material_effects;
#[path="__MIXINS__/source-material-traits.rs"] mod tconstruct_source_material_traits;
#[path="__MIXINS__/source-material-trait-select.rs"] mod tconstruct_source_material_trait_select;
#[path="__MIXINS__/source-upgrade-rules.rs"] mod tconstruct_source_upgrade_rules;
#[path="__MIXINS__/source-overslime.rs"] mod tconstruct_source_overslime;
#[path="__MIXINS__/source-mining-stack.rs"] mod tconstruct_source_mining_stack;
mod tconstruct_source_harvest_tags {
 // __HARVEST_ALIAS__
}
use std::collections::{HashMap,HashSet,BTreeMap};use horizon_items::items::ItemStack;use horizon_world::nbt::Tag;
mod tconstruct_world {
 use super::*;
 pub struct World{pub chunks:Chunks}
 pub struct Chunks {pub materials:Option<HashMap<String,Vec<Vec<u8>>>>,pub recipes:Option<BTreeMap<String,Vec<u8>>>,pub tags:Option<Result<HashMap<String,HashSet<String>>,()>>,pub blocks:Option<Result<HashMap<String,i32>,()>>,pub fluids:HashMap<String,HashSet<String>>,pub known_items:HashSet<String>,pub known_fluids:HashSet<String>}
 impl Chunks {
  pub fn __horizon_mixins_tconstruct_material_snapshot(&self)->&Option<HashMap<String,Vec<Vec<u8>>>>{&self.materials}
  pub fn __horizon_mixins_tconstruct_recipe_snapshot(&self)->&Option<BTreeMap<String,Vec<u8>>>{&self.recipes}
  pub fn __horizon_mixins_tconstruct_resolved_item_tags(&self)->&Option<Result<HashMap<String,HashSet<String>>,()>>{&self.tags}
  pub fn __horizon_mixins_tconstruct_resolved_block_ids(&self)->&Option<Result<HashMap<String,i32>,()>>{&self.blocks}
 }
}
mod tconstruct_tx {
 use super::*;
 #[derive(Debug,PartialEq)]pub enum SourceLookupError{UnsupportedRecipe}
 // __MERGERS__
 // __SET_DAMAGE__
}
mod tconstruct_source_tool_normalize {
 use super::*;
 pub fn prepare_for_tool_mutation(s:&ItemStack)->Result<ItemStack,&'static str>{
  if s.data().is_some_and(|d|d.wire.patch().is_some() || d.other("minecraft:tool").is_some()){Err("unsupported fixture transport")}else{Ok(s.clone())}
 }
 pub fn finalize_tool(s:&mut ItemStack,_:impl Fn(&str)->Option<i32>)->Result<(),&'static str>{
  if s.data().is_some_and(|d|d.other("minecraft:tool").is_some()){Err("unsupported fixture transport")}else{Ok(())}
 }
}
mod tconstruct_source_station_world {
 use super::*;
 // __ASSEMBLE__
 // __ASSEMBLE_STATS__
 // __HELMET__
 // __REBUILD__
}
mod tconstruct_source_builder_world {
 use super::*;
 pub fn supported_material(w:&tconstruct_world::World,m:&str)->Result<(bool,HashSet<String>),()> {
  let layers=w.chunks.materials.as_ref().ok_or(())?.get(&format!("tconstruct:tinkering/materials/stats/{}",m.strip_prefix("tconstruct:").ok_or(())?)).ok_or(())?;
  let stats=tconstruct_tx::source_merge_material_stats(layers).map_err(|_|())?;
  Ok((true,if stats.contains_key("tconstruct:plating_helmet"){HashSet::from(["tconstruct:helmet_plating".into()])}else{HashSet::new()}))
 }
}
mod candidate {
 use super::*;use tconstruct_source_melting_lookup::Catalog;use tconstruct_source_melting_recipes::CastingRecipe;
 fn with_catalog<T>(w:&tconstruct_world::World,f:impl FnOnce(&Catalog<'_>,&(),&())->T)->Option<T>{
  let c=&w.chunks;Some(f(&Catalog{recipes:c.recipes.as_ref()?,item_tags:c.tags.as_ref()?.as_ref().ok()?,fluid_tags:&c.fluids,known_items:&c.known_items,known_fluids:&c.known_fluids},&(),&()))
 }
 fn conditional_conditions(v:&[serde_json::Value],_:&(),_:&(),_:&HashSet<String>)->Result<bool,()>{if v.is_empty(){Ok(true)}else{Err(())}}
 fn material_allowed(_: &tconstruct_world::World,_:&Catalog<'_>,_:&impl Fn(&[serde_json::Value])->bool,v:Option<&serde_json::Value>,_:&str)->bool{v.is_none()}
 pub fn casting(world:&tconstruct_world::World,stack:&ItemStack,basin:bool,fluid:&str)->Option<(String,CastingRecipe,ItemStack)> {
 // __CANDIDATE__
 None
 }
}
fn stock_world()->tconstruct_world::World{
 let data:serde_json::Value=serde_json::from_str(include_str!("__FIXTURE__")).unwrap();
 let materials=data["materials"].as_object().unwrap().iter().map(|(k,v)|(k.clone(),vec![serde_json::to_vec(v).unwrap()])).collect();
 let recipes=data["recipes"].as_object().unwrap().iter().map(|(k,v)|(k.clone(),serde_json::to_vec(v).unwrap())).collect();
 tconstruct_world::World{chunks:tconstruct_world::Chunks{materials:Some(materials),recipes:Some(recipes),tags:Some(Ok(HashMap::from([("tconstruct:modifiable/armor".into(),HashSet::from(["tconstruct:plate_helmet".into()])),("tconstruct:modifiable/armor/worn".into(),HashSet::from(["tconstruct:plate_helmet".into()]))]))),blocks:Some(Ok(HashMap::new())),fluids:HashMap::from([("forge:molten_steel".into(),HashSet::from(["tconstruct:molten_steel".into()]))]),known_items:HashSet::from(["tconstruct:plate_helmet".into()]),known_fluids:HashSet::from(["tconstruct:molten_steel".into()])}}
}
fn iron_helmet(w:&tconstruct_world::World)->ItemStack{
 let def=w.chunks.materials.as_ref().unwrap()["tconstruct:tinkering/tool_definitions/plate_helmet"].last().unwrap();
 let mut s=tconstruct_source_station_world::assemble(w,def,"tconstruct:plate_helmet",&["tconstruct:iron".into(),"tconstruct:steel".into()],None,&mut None).unwrap();
 let max=match s.data().unwrap().other("minecraft:custom_data").unwrap().get("tic_max_damage").unwrap(){Tag::Int(n)=>*n as u32,_=>panic!()};
 tconstruct_tx::fixed_set_damage(&mut s,max);s
}
#[test]fn stock_broken_iron_helmet_casts_to_usable_steel_and_preserves_until_complete(){
 let w=stock_world();let original=iron_helmet(&w);
 let def=w.chunks.materials.as_ref().unwrap()["tconstruct:tinkering/tool_definitions/plate_helmet"].last().unwrap();
 assert!(tconstruct_source_station_world::rebuild_material_swap(&w,def,&original,&["tconstruct:steel".into(),"tconstruct:steel".into()],0,true,&mut None).is_none());
 let (id,r,out)=candidate::casting(&w,&original,false,"tconstruct:molten_steel").expect("actual stock casting path");
 assert_eq!(r.fluid.amount,270);assert!(r.consumes_cast);
 assert_eq!(out.damage,63);let custom=out.data().unwrap().other("minecraft:custom_data").unwrap();assert_eq!(custom.get("tic_broken"),Some(&Tag::Byte(0)));assert_eq!(custom.get("tic_max_damage"),Some(&Tag::Int(382)));
 assert_eq!(custom.get("tic_materials"),Some(&Tag::List(vec![Tag::String("tconstruct:steel".into());2])));
 let levels=tconstruct_source_material_traits::levels(&w,&out).unwrap().levels;
 assert_eq!(levels,BTreeMap::from([("tconstruct:ductile".into(),2)]));
 let mut c=tconstruct_source_melter_state::Casting::new();c.input=Some(original.clone());
 assert!(c.prepare(&id,r.fluid.amount,r.cooling_time,out.clone(),r.consumes_cast,r.switch_slots));
 assert!(c.tank.insert("tconstruct:molten_steel",269));assert!(!c.tick(true));assert_eq!(c.input,Some(original.clone()));
 assert!(c.tank.insert("tconstruct:molten_steel",1));let mut c=tconstruct_source_melter_state::Casting::from_nbt(&c.to_nbt()).unwrap();
 assert_eq!(c.input,Some(original));assert!(!c.tick(false));
 for _ in 1..r.cooling_time{assert!(!c.tick(true));}assert!(c.tick(true));assert_eq!(c.output,Some(out));assert!(c.input.is_none());assert_eq!(c.tank.amount,0);
}
#[test]fn stock_exception_rejects_unknown_item_data_wire_and_unsupported_traits(){
 let mut w=stock_world();let original=iron_helmet(&w);
 for key in ["minecraft:custom_name","test:extra"]{let mut s=original.clone();s.data_mut().set_other(key,Some(Tag::String("keep".into())));let before=s.clone();assert!(candidate::casting(&w,&s,false,"tconstruct:molten_steel").is_none());assert_eq!(s,before);}
 let mut wire=original.clone();wire.data_mut().wire=horizon_items::item_data::Wire::new(original.clone(),std::sync::Arc::<[u8]>::from([1,2,3]));assert!(candidate::casting(&w,&wire,false,"tconstruct:molten_steel").is_none());wire.damage-=1;assert!(wire.wire_patch().is_none());assert!(candidate::casting(&w,&wire,false,"tconstruct:molten_steel").is_none());
 w.chunks.materials.as_mut().unwrap().insert("tconstruct:tinkering/materials/traits/steel".into(),vec![br#"{"default":[{"name":"test:unknown","level":1}]}"#.to_vec()]);assert!(candidate::casting(&w,&original,false,"tconstruct:molten_steel").is_none());
}
#[test]fn repaired_damage_uses_new_maximum_and_rejects_inconsistent_flags(){
 let w=stock_world();let mut s=iron_helmet(&w);let old=s.damage;
 for max in [old/2,old,old*2]{assert_eq!(tconstruct_source_melter_state::casting_swap_damage_after_rebuild(&s,max),Some(max));}
 s.damage-=1;assert_eq!(tconstruct_source_melter_state::casting_swap_broken(&s),None);
 tconstruct_tx::fixed_set_damage(&mut s,old-1);assert_eq!(tconstruct_source_melter_state::casting_swap_damage_after_rebuild(&s,old/2),Some(old/2));assert_eq!(tconstruct_source_melter_state::casting_swap_damage_after_rebuild(&s,old*2),Some(old-1));
}

#[test]fn broken_composite_and_missing_stock_context_remain_closed(){
 let mut w=stock_world();let original=iron_helmet(&w);
 let key="tconstruct:tools/materials/casting/steel";
 let bytes=w.chunks.recipes.as_mut().unwrap().get_mut(key).unwrap();let mut v:serde_json::Value=serde_json::from_slice(bytes).unwrap();v["input"]=serde_json::json!("tconstruct:iron");*bytes=serde_json::to_vec(&v).unwrap();
 assert!(candidate::casting(&w,&original,false,"tconstruct:molten_steel").is_none());
 let mut w=stock_world();w.chunks.materials.as_mut().unwrap().remove("tconstruct:tinkering/materials/traits/iron");assert!(candidate::casting(&w,&original,false,"tconstruct:molten_steel").is_none());
 let w=stock_world();assert!(candidate::casting(&w,&original,true,"tconstruct:molten_steel").is_none());assert!(candidate::casting(&w,&original,false,"minecraft:water").is_none());
}
