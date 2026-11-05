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
