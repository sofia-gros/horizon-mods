#[path="__MIXINS__/source-smeltery-fuel.rs"] mod tconstruct_source_smeltery_fuel;
#[path="__MIXINS__/source-melter-state.rs"] mod tconstruct_source_melter_state;
#[path="__MIXINS__/source-smeltery-tank.rs"] mod tconstruct_source_smeltery_tank;
#[path="__MIXINS__/source-smeltery-state.rs"] mod tconstruct_source_smeltery_state;
#[path="__MIXINS__/source-fluid-container.rs"] mod tconstruct_source_fluid_container;
#[path="__MIXINS__/source-fluid-display.rs"] mod tconstruct_source_fluid_display;
#[path="__PLUTO__"] mod pluto_stack_fluid;
use std::collections::{HashMap,HashSet};
use horizon_world::nbt::Tag;
use horizon_items::items::ItemStack;
type Pos=(i32,i32,i32);
type Tags=HashMap<String,HashSet<String>>;
struct Info{name:String} struct Facts;
impl Facts {fn get(&self,id:usize)->Info {Info{name: match id {1=>"tconstruct:smeltery_controller",2=>"tconstruct:foundry_controller",_=>"minecraft:air"}.into()}}}
struct Items; impl Items {fn get(&self,_:&str)->Option<()> {Some(())}}
struct Res; impl Res {fn facts(&self)->Facts{Facts} fn items(&self)->Items{Items}}
struct Chunks {owners:HashMap<Pos,Pos>,fluids:Option<Result<(HashSet<String>,Tags),()>>,tags:Option<Result<Tags,()>>}
impl Chunks {
 fn __horizon_mixins_tconstruct_smeltery_owners(&self)->&HashMap<Pos,Pos>{&self.owners}
 fn __horizon_mixins_tconstruct_tank_fluid_context(&self)->&Option<Result<(HashSet<String>,Tags),()>>{&self.fluids}
 fn __horizon_mixins_tconstruct_resolved_item_tags(&self)->&Option<Result<Tags,()>>{&self.tags}
}
struct World {chunks:Chunks,raw:Tag,formed:bool,fail:bool,controller:usize,writes:usize}
impl World {fn block(&self,_:i32,_:i32,_:i32)->usize{self.controller}}
struct Game<'a> {world:World,res:Res,held:Option<ItemStack>,gifts:Vec<ItemStack>,creative:bool,lan:Option<()>,lan_acting_uuid:Option<()>,_life:std::marker::PhantomData<&'a()>}
impl Game<'_> {fn ensure_datapacks(&mut self){} fn held(&self)->Option<&ItemStack>{self.held.as_ref()} fn held_mut(&mut self)->&mut Option<ItemStack>{&mut self.held} fn give_or_drop(&mut self,s:ItemStack){self.gifts.push(s)}}
mod tconstruct_source_smeltery_world {use super::*;pub(super) fn detect(g:&Game<'_>,_:Pos)->Option<([i32;3],[i32;3],u32)>{g.world.formed.then_some(([-2,-1,-1],[0,1,1],4000))}}
mod store {use super::*;pub(super) fn read(w:&World,_:Pos,_:&str)->Option<Tag>{Some(w.raw.clone())} pub(super) fn write(w:&mut World,_:Pos,_:&str,t:Tag)->bool{if w.fail{return false}w.raw=t;w.writes+=1;true}}
mod runtime {
use super::*;use super::tconstruct_source_smeltery_state::Smeltery;
// __PRODUCTION_PATCH__
// __PRODUCTION_ROUTE__
}
fn fixture(item:&str,amount:u32)->Game<'static>{
 let mut state=tconstruct_source_smeltery_state::Smeltery::new();state.tank.capacity=4000;
 if amount>0 {assert!(state.tank.insert("minecraft:water",amount));}
 Game{world:World{chunks:Chunks{owners:HashMap::from([((-2,0,0),(-1,0,0))]),fluids:Some(Ok((HashSet::from(["minecraft:water".into(),"minecraft:lava".into()]),Tags::new()))),tags:Some(Ok(Tags::new()))},raw:state.to_nbt(),formed:true,fail:false,controller:1,writes:0},res:Res,held:Some(ItemStack::new(item,1)),gifts:vec![],creative:false,lan:None,lan_acting_uuid:None,_life:std::marker::PhantomData}
}
#[test]fn full_tank_and_empty_tank_consume_container_action_without_mutation(){
 for (item,amount) in [("minecraft:water_bucket",4000),("minecraft:water_bucket",3500),("minecraft:lava_bucket",4000),("minecraft:bucket",0)] {
  let mut g=fixture(item,amount);let raw=g.world.raw.clone();
  assert!(runtime::interact_drain(&mut g,(-2,0,0)));
  assert_eq!(g.world.raw,raw);assert_eq!(g.world.writes,0);assert_eq!(g.held.as_ref().unwrap().item.as_ref(),item);assert!(g.gifts.is_empty());
 }
}
#[test]fn noncontainers_and_unformed_drains_still_pass(){
 let mut g=fixture("tconstruct:seared_faucet",4000);assert!(!runtime::interact_drain(&mut g,(-2,0,0)));assert_eq!(g.world.writes,0);
 let mut g=fixture("minecraft:water_bucket",4000);g.world.formed=false;assert!(!runtime::interact_drain(&mut g,(-2,0,0)));
 let mut g=fixture("minecraft:water_bucket",4000);g.world.chunks.owners.clear();assert!(!runtime::interact_drain(&mut g,(-2,0,0)));
}
#[test]fn successful_and_rejected_save_paths_preserve_existing_item_transaction(){
 let mut g=fixture("minecraft:water_bucket",0);assert!(runtime::interact_drain(&mut g,(-2,0,0)));assert_eq!(g.world.writes,1);assert_eq!(g.held.as_ref().unwrap().item.as_ref(),"minecraft:bucket");
 assert_eq!(tconstruct_source_smeltery_state::Smeltery::from_nbt(&g.world.raw).unwrap().tank.total(),1000);
 let mut g=fixture("minecraft:water_bucket",0);g.world.fail=true;let raw=g.world.raw.clone();assert!(!runtime::interact_drain(&mut g,(-2,0,0)));assert_eq!(raw,g.world.raw);assert_eq!(g.held.as_ref().unwrap().item.as_ref(),"minecraft:water_bucket");assert!(g.gifts.is_empty());
}

fn extended_controller(game:&mut Game<'_>) {
 let mut state=tconstruct_source_smeltery_state::Smeltery::from_nbt(&game.world.raw).unwrap();
 state.slots=vec![Some(ItemStack::new("minecraft:stone",3))];state.progress=vec![17];state.fuel=33;state.temperature=800;
 let mut raw=state.to_nbt();
 if let Tag::Compound(fields)=&mut raw {
  fields.insert("future".into(),Tag::Long(7));fields.insert("heat_codes".into(),Tag::IntArray(vec![5]));
  fields.insert("fuel_view".into(),Tag::compound([("future_fuel",Tag::Int(8))]));
  if let Some(Tag::List(slots))=fields.get_mut(&"slots".into()) {if let Tag::Compound(item)=&mut slots[0]{item.insert("future_item".into(),Tag::Int(9));}}
 }
 game.world.raw=raw;
}
#[test]fn drain_held_preserves_raw_controller_and_item_fields(){
 for controller in [1,2] {for draining in [false,true] {
  let mut game=fixture(if draining{"minecraft:bucket"}else{"minecraft:water_bucket"},if draining{1000}else{0});game.world.controller=controller;extended_controller(&mut game);
  let before=game.world.raw.clone();assert!(runtime::interact_drain(&mut game,(-2,0,0)));
  let state=tconstruct_source_smeltery_state::Smeltery::from_nbt(&game.world.raw).unwrap();assert_eq!(state.tank.total(),if draining{0}else{1000});
  let Tag::Compound(fields)=&before else{panic!()};
  for (key,value) in fields {if key.to_string_lossy()!="tank"{assert_eq!(game.world.raw.get(&key.to_string_lossy()),Some(value));}}
  assert_eq!(game.world.writes,1);assert!(game.gifts.is_empty());
  assert_eq!(&*game.held.as_ref().unwrap().item,if draining{"minecraft:water_bucket"}else{"minecraft:bucket"});
 }}
}
#[test]fn drain_held_refuses_noncanonical_tanks_and_retained_wire(){
 use horizon_items::item_data::Wire;use horizon_world::nbt::JavaString;
 for mode in 0..6 {
  let mut game=fixture("minecraft:water_bucket",1000);
  if mode<4 {if let Tag::Compound(fields)=&mut game.world.raw {if let Some(Tag::Compound(tank))=fields.get_mut(&"tank".into()){
   match mode {
    0=>{tank.insert("Capacity".into(),Tag::Int(4000));},
    1=>{tank.insert("Amount".into(),Tag::Int(1000));},
    2|3=>{if let Some(Tag::List(rows))=tank.get_mut(&"fluids".into()){if let Tag::Compound(row)=&mut rows[0]{if mode==2{row.insert("Amount".into(),Tag::Int(1000));}else{row.insert("id".into(),Tag::String(JavaString(vec![0xd800])));}}}},
    _=>unreachable!(),
   }
  }}} else {
   let held=game.held.as_mut().unwrap();held.data_mut().wire=Wire::new(held.clone(),std::sync::Arc::<[u8]>::from([1,2,3]));
   if mode==5{held.data_mut().set_other("minecraft:custom_name",Some(Tag::String("Changed".into())));}
  }
  let before=(game.world.raw.clone(),game.held.clone());assert!(!runtime::interact_drain(&mut game,(-2,0,0)));
  assert_eq!((game.world.raw,game.held),before);assert_eq!(game.world.writes,0);assert!(game.gifts.is_empty());
 }
}
#[test]fn drain_held_preservation_save_failure_and_creative_paths(){
 for draining in [false,true] {for fail in [false,true] {
  let mut game=fixture(if draining{"minecraft:bucket"}else{"minecraft:water_bucket"},if draining{1000}else{0});extended_controller(&mut game);game.creative=true;game.world.fail=fail;
  let raw=game.world.raw.clone();let held=game.held.clone();assert_eq!(runtime::interact_drain(&mut game,(-2,0,0)),!fail);
  assert_eq!(game.held,held);assert_eq!(game.gifts.len(),usize::from(draining && !fail));
  if fail{assert_eq!(game.world.raw,raw);assert_eq!(game.world.writes,0);}else{assert_eq!(game.world.writes,1);assert_eq!(game.world.raw.get("slots"),raw.get("slots"));}
 }}
}


fn extend_drain_tank(game:&mut Game<'_>,row_extensions:bool){
 if let Tag::Compound(fields)=&mut game.world.raw{if let Some(Tag::Compound(tank))=fields.get_mut(&"tank".into()){
  tank.insert("future_tank".into(),Tag::Int(7));
  if row_extensions{if let Some(Tag::List(rows))=tank.get_mut(&"fluids".into()){for row in rows{if let Tag::Compound(fields)=row{fields.insert("future_row".into(),Tag::Int(8));}}}}
 }}
}
#[test]fn drain_held_preserves_native_tank_and_row_extensions(){
 for controller in [1,2]{for draining in [false,true]{
  let mut game=fixture(if draining{"minecraft:bucket"}else{"minecraft:water_bucket"},2000);game.world.controller=controller;extended_controller(&mut game);extend_drain_tank(&mut game,true);
  let before=game.world.raw.clone();assert!(runtime::interact_drain(&mut game,(-2,0,0)));
  let state=tconstruct_source_smeltery_state::Smeltery::from_nbt(&game.world.raw).unwrap();assert_eq!(state.tank.total(),if draining{1000}else{3000});
  assert_eq!(game.world.raw.get("slots"),before.get("slots"));assert_eq!(game.world.raw.get("future"),before.get("future"));
  let tank=game.world.raw.get("tank").unwrap();assert_eq!(tank.get("future_tank"),Some(&Tag::Int(7)));
  let Tag::List(rows)=tank.get("fluids").unwrap() else{panic!()};assert_eq!(rows[0].get("future_row"),Some(&Tag::Int(8)));
  assert_eq!(game.world.writes,1);assert_eq!(&*game.held.as_ref().unwrap().item,if draining{"minecraft:water_bucket"}else{"minecraft:bucket"});
 }}
 // Top-level tank extensions survive an empty/refilled tank as well.
 let mut game=fixture("minecraft:bucket",1000);extend_drain_tank(&mut game,false);
 assert!(runtime::interact_drain(&mut game,(-2,0,0)));assert!(runtime::interact_drain(&mut game,(-2,0,0)));
 assert_eq!(game.world.raw.get("tank").unwrap().get("future_tank"),Some(&Tag::Int(7)));assert_eq!(game.world.writes,2);
}
#[test]fn drain_held_opaque_row_removal_and_failed_save_leave_items_unchanged(){
 for fail in [false,true]{let mut game=fixture("minecraft:bucket",if fail{2000}else{1000});extend_drain_tank(&mut game,true);game.world.fail=fail;
  let raw=game.world.raw.clone();let held=game.held.clone();assert!(!runtime::interact_drain(&mut game,(-2,0,0)));
  assert_eq!(game.world.raw,raw);assert_eq!(game.held,held);assert_eq!(game.world.writes,0);assert!(game.gifts.is_empty());
 }
}
