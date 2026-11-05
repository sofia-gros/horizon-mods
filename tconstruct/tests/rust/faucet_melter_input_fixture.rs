extern crate self as horizon_client;
pub mod containers { pub enum BlockEntity { Chest(Vec<Option<horizon_items::items::ItemStack>>) } }
#[path="__MIXINS__/source-melter-state.rs"] mod tconstruct_source_melter_state;
#[path="__MIXINS__/source-smeltery-fuel.rs"] mod tconstruct_source_smeltery_fuel;
#[path="__MIXINS__/source-fluid-container.rs"] mod tconstruct_source_fluid_container;
#[path="__MIXINS__/source-fluid-display.rs"] mod tconstruct_source_fluid_display;
#[path="__MIXINS__/source-faucet-tank.rs"] mod tconstruct_source_faucet_tank;
#[path="__PLUTO__"] mod pluto_stack_fluid;
use std::collections::HashMap;
use horizon_world::nbt::Tag;
use tconstruct_source_melter_state::{Faucet, Melter, Tank};
type Pos = (i32,i32,i32);
struct Info { name: String }
struct Facts;
impl Facts {
 fn get(&self,id:usize)->Info { Info { name: match id {
  1=>"tconstruct:seared_fuel_tank", 2=>"tconstruct:seared_fuel_gauge",
  3=>"tconstruct:scorched_fuel_tank", 4=>"tconstruct:scorched_fuel_gauge",
  5=>"tconstruct:seared_ingot_tank", 6=>"tconstruct:seared_ingot_gauge",
  7=>"tconstruct:scorched_ingot_tank", 8=>"tconstruct:scorched_ingot_gauge",
  9=>"tconstruct:seared_melter", 10=>"tconstruct:seared_faucet",
  11=>"tconstruct:seared_drain", _=>"minecraft:air",
 }.into() } }
}
struct Res;
impl Res { fn facts(&self)->Facts { Facts } }
struct World {
 blocks:HashMap<Pos,usize>, block_entities:HashMap<Pos,containers::BlockEntity>,
 chunks:HashMap<(i32,i32),()>, tags:HashMap<(Pos,String),Tag>, fail:bool,
}
impl World { fn block(&self,x:i32,y:i32,z:i32)->usize { *self.blocks.get(&(x,y,z)).unwrap_or(&0) } }
struct Game<'a> { res:Res, world:World, notifications:Vec<Pos>, _life:std::marker::PhantomData<&'a()> }
impl Game<'_> { fn container_changed(&mut self, pos:Pos) { self.notifications.push(pos); } }
mod tconstruct_source_melter_store {
 use super::*;
 pub(super) fn read(world:&World,pos:Pos,key:&str)->Option<Tag> { world.tags.get(&(pos,key.into())).cloned() }
 pub(super) fn write_many(world:&mut World, changes:&[(Pos,&str,Tag)])->bool {
  if world.fail || changes.iter().any(|(p,k,_)| !world.tags.contains_key(&(*p,(*k).into()))) { return false; }
  for (p,k,raw) in changes { world.tags.insert((*p,(*k).into()),raw.clone()); }
  true
 }
}
mod tconstruct_source_machine_runtime {
 use super::*;
 use super::containers::BlockEntity;
 use super::tconstruct_source_melter_store as store;
 // __PRODUCTION_ROUTE__
 #[test]
 fn all_eight_tanks_fill_melter_without_rewriting_slots_fuel_or_extensions() {
  for source_id in 1..=8 {
   let (mut g,p,s,t,key,capacity)=fixture(source_id);
   let raw=store::read(&g.world,t,"tconstruct:melter").unwrap();
   let live=g.world.block_entities.get(&t).unwrap();
   let BlockEntity::Chest(slots)=live;
   let original_item=slots[0].as_ref().unwrap().item.clone();
   assert_eq!(tank_to_melter(&mut g,p,s,t,Faucet::new(),true),Some(true));
   let saved=store::read(&g.world,t,"tconstruct:melter").unwrap();
   for field in ["slots","progress","phase","fuel","fuel_quality","temperature","rate","heat_codes"] { assert_eq!(saved.get(field),raw.get(field)); }
   assert_eq!(melter(&g,t).tank.amount,10);
   assert_eq!(Tank::from_nbt(&store::read(&g.world,s,key).unwrap(),capacity).unwrap().amount,90);
   assert_eq!(faucet(&g,p).pending.amount,80);
   assert_eq!(g.notifications,vec![s,t]);
   let BlockEntity::Chest(slots)=g.world.block_entities.get(&t).unwrap();
   assert_eq!(slots[0].as_ref().unwrap().item,original_item);
   for _ in 0..8 { let f=faucet(&g,p); assert_eq!(tank_to_melter(&mut g,p,s,t,f,false),Some(true)); }
   let f=faucet(&g,p); assert_eq!(f.pending.amount,0);
   assert_eq!(tank_to_melter(&mut g,p,s,t,f,false),Some(true));
   assert_eq!((melter(&g,t).tank.amount,faucet(&g,p).pending.amount),(100,80));
  }
 }
 #[test]
 fn failed_save_opaque_tanks_unloaded_chunks_and_missing_entities_wait() {
  for failure in 0..8 {
   let (mut g,p,s,t,key,_)=fixture(1);
   match failure {
    0=>g.world.fail=true,
    1=>{ g.world.block_entities.remove(&t); },
    2=>{ g.world.block_entities.remove(&s); },
    3=>{ g.world.chunks.remove(&(-1,0)); },
    4=>{ if let Tag::Compound(fields)=g.world.tags.get_mut(&(t,"tconstruct:melter".into())).unwrap() { if let Some(Tag::Compound(tank))=fields.get_mut(&"tank".into()) { tank.insert("unknown".into(),Tag::Int(7)); } } },
    5=>{ if let Tag::Compound(fields)=g.world.tags.get_mut(&(s,key.into())).unwrap() { fields.insert("unknown".into(),Tag::Int(7)); } },
    6=>{ g.world.tags.remove(&(t,"tconstruct:melter".into())); },
    _=>{ let mut raw=Faucet::new().to_nbt(); if let Tag::Compound(fields)=&mut raw { fields.insert("active".into(),Tag::Int(1)); } g.world.tags.insert((p,"tconstruct:faucet".into()),raw); },
   }
   let before=g.world.tags.clone();
   assert_eq!(tank_to_melter(&mut g,p,s,t,Faucet::new(),true),Some(false));
   assert_eq!(g.world.tags,before); assert!(g.notifications.is_empty());
  }
  let (mut g,p,s,t,_,_)=fixture(1);
  assert_eq!(tank_to_melter(&mut g,p,t,t,Faucet::new(),true),Some(false));
  assert_eq!(tank_to_melter(&mut g,t,s,t,Faucet::new(),true),Some(false));
 }
 #[test]
 fn remaining_capacity_and_incompatible_fluid_do_not_overfill_or_mix() {
  for (fluid,amount,expected) in [("x:copper",2155,2160),("x:copper",2160,2160),("minecraft:water",10,10)] {
   let (mut g,p,s,t,key,capacity)=fixture(1);
   let mut state=melter(&g,t); assert!(state.tank.insert(fluid,amount));
   let raw=store::read(&g.world,t,"tconstruct:melter").unwrap();
   g.world.tags.insert((t,"tconstruct:melter".into()),tconstruct_source_faucet_tank::patch_melter_tank(&raw,&state.tank).unwrap());
   assert_eq!(tank_to_melter(&mut g,p,s,t,Faucet::new(),true),Some(true));
   assert_eq!(melter(&g,t).tank.amount,expected);
   assert_eq!(Tank::from_nbt(&store::read(&g.world,s,key).unwrap(),capacity).unwrap().amount,180-(expected-amount));
   assert_eq!(faucet(&g,p).pending.amount,0);
  }
 }
 #[test]
 fn base_potion_identity_is_preserved_and_unsupported_tags_wait() {
  let first=Tag::compound([("Potion",Tag::String("minecraft:swiftness".into()))]);
  let second=Tag::compound([("Potion",Tag::String("minecraft:healing".into()))]);
  let (mut g,p,s,t,key,capacity)=fixture(1);
  let mut input=Tank::new(capacity); assert!(input.insert_tagged("tconstruct:potion",Some(&first),180));
  g.world.tags.insert((s,key.into()),input.to_nbt());
  assert_eq!(tank_to_melter(&mut g,p,s,t,Faucet::new(),true),Some(true));
  assert_eq!(melter(&g,t).tank.tag,Some(first.clone())); assert_eq!(faucet(&g,p).pending.tag,Some(first));
  let mut state=melter(&g,t); state.tank.tag=Some(second);
  let raw=store::read(&g.world,t,"tconstruct:melter").unwrap();
  g.world.tags.insert((t,"tconstruct:melter".into()),tconstruct_source_faucet_tank::patch_melter_tank(&raw,&state.tank).unwrap());
  let f=faucet(&g,p); assert_eq!(tank_to_melter(&mut g,p,s,t,f,false),Some(true));
  assert_eq!(melter(&g,t).tank.amount,10); assert_eq!(faucet(&g,p).pending.amount,80);
  for tag in [Tag::compound([("Potion",Tag::String("minecraft:swiftness".into())),("CustomPotionColor",Tag::Int(7))]),Tag::compound([("Potion",Tag::String("unknown:custom".into()))])] {
   let (mut g,p,s,t,key,capacity)=fixture(1); let mut input=Tank::new(capacity);
   assert!(input.insert_tagged("tconstruct:potion",Some(&tag),180)); g.world.tags.insert((s,key.into()),input.to_nbt());
   let before=g.world.tags.clone(); assert_eq!(tank_to_melter(&mut g,p,s,t,Faucet::new(),true),Some(false)); assert_eq!(g.world.tags,before);
  }
 }
 #[test]
 fn reserved_packet_finishes_after_removed_source_and_stop_prevents_refill() {
  for removed in [false,true] {
   let (mut g,p,s,t,key,capacity)=fixture(1);
   assert_eq!(tank_to_melter(&mut g,p,s,t,Faucet::new(),true),Some(true));
   if removed { g.world.blocks.remove(&s); g.world.block_entities.remove(&s); }
   else {
    let mut input=Tank::new(capacity); let tag=Tag::compound([("unknown",Tag::Int(7))]);
    assert!(input.insert_tagged("x:copper",Some(&tag),90)); g.world.tags.insert((s,key.into()),input.to_nbt());
    let f=faucet(&g,p); assert!(tconstruct_source_faucet_tank::stop(&mut g,p,f));
   }
   for _ in 0..9 { let f=faucet(&g,p); assert_eq!(tank_to_melter(&mut g,p,s,t,f,false),Some(true)); }
   assert_eq!(melter(&g,t).tank.amount,90); assert_eq!(faucet(&g,p).pending.amount,0); assert!(!faucet(&g,p).active);
  }
 }
}
fn faucet(g:&Game<'_>,p:Pos)->Faucet { Faucet::from_nbt(g.world.tags.get(&(p,"tconstruct:faucet".into())).unwrap()).unwrap() }
fn melter(g:&Game<'_>,p:Pos)->Melter { Melter::from_nbt(g.world.tags.get(&(p,"tconstruct:melter".into())).unwrap()).unwrap() }
fn fixture(id:usize)->(Game<'static>,Pos,Pos,Pos,&'static str,u32) {
 let (p,s,t)=((0,1,0),(-1,1,0),(0,0,0));
 let (key,capacity)=tconstruct_source_faucet_tank::tank_kind(&Facts.get(id).name).unwrap();
 let mut input=Tank::new(capacity); assert!(input.insert("x:copper",180));
 let mut state=Melter::new(); state.fuel=37; state.temperature=900; state.progress=[-1,5,0]; state.phase=2; state.fuel_quality=100; state.rate=7;
 state.slots[0]=Some(horizon_items::items::ItemStack::new("minecraft:copper_ore",1));
 let mut raw=state.to_nbt(); if let Tag::Compound(fields)=&mut raw {
  fields.insert("heat_codes".into(),Tag::IntArray(vec![7,8,9]));
  if let Some(Tag::List(slots))=fields.get_mut(&"slots".into()) { if let Tag::Compound(slot)=&mut slots[0] { slot.insert("opaque".into(),Tag::Int(77)); } }
 }
 let tags=HashMap::from([((s,key.into()),input.to_nbt()),((t,"tconstruct:melter".into()),raw),((p,"tconstruct:faucet".into()),Faucet::new().to_nbt())]);
 let world=World{blocks:HashMap::from([(s,id),(t,9),(p,10)]),block_entities:HashMap::from([(s,containers::BlockEntity::Chest(vec![])),(t,containers::BlockEntity::Chest(vec![Some(horizon_items::items::ItemStack::new("minecraft:iron_ore",1)),None,None]))]),chunks:HashMap::from([((-1,0),()),((0,0),())]),tags,fail:false};
 (Game{res:Res,world,notifications:vec![],_life:std::marker::PhantomData},p,s,t,key,capacity)
}
