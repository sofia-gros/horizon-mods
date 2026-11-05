extern crate self as horizon_client;
pub mod containers { pub enum BlockEntity { Chest(Vec<Option<horizon_items::items::ItemStack>>) } }
#[path="__MIXINS__/source-fluid-cannon.rs"] mod tconstruct_source_fluid_cannon;
#[path="__MIXINS__/source-casting-tank.rs"] mod tconstruct_source_casting_tank;
#[path="__MIXINS__/source-melter-state.rs"] mod tconstruct_source_melter_state;
#[path="__MIXINS__/source-smeltery-state.rs"] mod tconstruct_source_smeltery_state;
#[path="__MIXINS__/source-smeltery-tank.rs"] mod tconstruct_source_smeltery_tank;
#[path="__MIXINS__/source-smeltery-fuel.rs"] mod tconstruct_source_smeltery_fuel;
#[path="__MIXINS__/source-fluid-container.rs"] mod tconstruct_source_fluid_container;
#[path="__MIXINS__/source-fluid-display.rs"] mod tconstruct_source_fluid_display;
#[path="__MIXINS__/source-faucet-tank.rs"] mod tconstruct_source_faucet_tank;
#[path="__PLUTO__"] mod pluto_stack_fluid;
use std::collections::HashMap;
use horizon_world::nbt::Tag;
use tconstruct_source_melter_state::{Faucet, Melter, Tank};
use tconstruct_source_smeltery_state::Smeltery;
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
  11=>"tconstruct:seared_drain", 12=>"tconstruct:smeltery_controller",
  13=>"tconstruct:foundry_controller", 14=>"tconstruct:scorched_drain", 15=>"tconstruct:seared_duct", 16=>"tconstruct:seared_casting_tank", 17=>"tconstruct:seared_lantern",
  18=>"tconstruct:scorched_lantern", 19=>"tconstruct:seared_fluid_cannon", _=>"minecraft:air",
 }.into() } }
}
struct Res;
impl Res { fn facts(&self)->Facts { Facts } }
struct Chunks { loaded:HashMap<(i32,i32),()>, owners:HashMap<Pos,Pos> }
impl Chunks {
 fn contains_key(&self,pos:&(i32,i32))->bool { self.loaded.contains_key(pos) }
 fn __horizon_mixins_tconstruct_smeltery_owners(&self)->&HashMap<Pos,Pos> { &self.owners }
}
mod tconstruct_source_smeltery_world {
 use super::*;
 pub(super) fn detect(game:&Game<'_>,_:Pos)->Option<([i32;3],[i32;3],u32)> {
  game.world.formed.then_some(([-3,0,-1],[-1,2,1],4000))
 }
}
struct World {
 blocks:HashMap<Pos,usize>, block_entities:HashMap<Pos,containers::BlockEntity>,
 chunks:Chunks, tags:HashMap<(Pos,String),Tag>, fail:bool, formed:bool,
}
impl World { fn block(&self,x:i32,y:i32,z:i32)->usize { *self.blocks.get(&(x,y,z)).unwrap_or(&0) } }
struct Game<'a> { res:Res, world:World, notifications:Vec<Pos>, process_calls:usize, process_fault:u8, signals:usize, _life:std::marker::PhantomData<&'a()> }
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
 fn notify_casting_comparator(game:&mut Game<'_>,_:Pos,before:i32,after:i32) {
  if before != after { game.signals += 1; }
 }
 fn process_casting_tank(game:&mut Game<'_>,state:&mut super::tconstruct_source_casting_tank::CastingTank)->bool {
  game.process_calls += 1;
  let changed=state.process(&|_|true,&|_|true,None,false);
  match game.process_fault {
   1=>{ let BlockEntity::Chest(slots)=game.world.block_entities.get_mut(&(-1,1,0)).unwrap(); slots[0]=None; },
   2=>{ game.world.blocks.remove(&(0,0,0)); },
   3=>{ game.world.chunks.loaded.remove(&(-1,0)); },
   4=>{ game.world.block_entities.remove(&(0,0,0)); },
   _=>(),
  }
  changed
 }
 // __PRODUCTION_ROUTE__
 #[test]
 fn all_eight_tanks_fill_melter_without_rewriting_slots_fuel_or_extensions() {
  for source_id in 1..=8 {
   let (mut g,p,s,t,key,capacity)=fixture(source_id);
   let raw=store::read(&g.world,t,"tconstruct:melter").unwrap();
   let live=g.world.block_entities.get(&t).unwrap();
   let BlockEntity::Chest(slots)=live;
   let original_item=slots[0].as_ref().unwrap().item.clone();
   assert_eq!(melter_lantern_transfer(&mut g,p,s,t,Faucet::new(),true),Some(true));
   let saved=store::read(&g.world,t,"tconstruct:melter").unwrap();
   for field in ["slots","progress","phase","fuel","fuel_quality","temperature","rate","heat_codes"] { assert_eq!(saved.get(field),raw.get(field)); }
   assert_eq!(melter(&g,t).tank.amount,10);
   assert_eq!(Tank::from_nbt(&store::read(&g.world,s,key).unwrap(),capacity).unwrap().amount,90);
   assert_eq!(faucet(&g,p).pending.amount,80);
   assert_eq!(g.notifications,vec![s,t]);
   let BlockEntity::Chest(slots)=g.world.block_entities.get(&t).unwrap();
   assert_eq!(slots[0].as_ref().unwrap().item,original_item);
   for _ in 0..8 { let f=faucet(&g,p); assert_eq!(melter_lantern_transfer(&mut g,p,s,t,f,false),Some(true)); }
   let f=faucet(&g,p); assert_eq!(f.pending.amount,0);
   assert_eq!(melter_lantern_transfer(&mut g,p,s,t,f,false),Some(true));
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
    3=>{ g.world.chunks.loaded.remove(&(-1,0)); },
    4=>{ if let Tag::Compound(fields)=g.world.tags.get_mut(&(t,"tconstruct:melter".into())).unwrap() { if let Some(Tag::Compound(tank))=fields.get_mut(&"tank".into()) { tank.insert("unknown".into(),Tag::Int(7)); } } },
    5=>{ if let Tag::Compound(fields)=g.world.tags.get_mut(&(s,key.into())).unwrap() { fields.insert("unknown".into(),Tag::Int(7)); } },
    6=>{ g.world.tags.remove(&(t,"tconstruct:melter".into())); },
    _=>{ let mut raw=Faucet::new().to_nbt(); if let Tag::Compound(fields)=&mut raw { fields.insert("active".into(),Tag::Int(1)); } g.world.tags.insert((p,"tconstruct:faucet".into()),raw); },
   }
   let before=g.world.tags.clone();
   assert_eq!(melter_lantern_transfer(&mut g,p,s,t,Faucet::new(),true),Some(false));
   assert_eq!(g.world.tags,before); assert!(g.notifications.is_empty());
  }
  let (mut g,p,s,t,_,_)=fixture(1);
  assert_eq!(melter_lantern_transfer(&mut g,p,t,t,Faucet::new(),true),Some(false));
  assert_eq!(melter_lantern_transfer(&mut g,t,s,t,Faucet::new(),true),Some(false));
 }
 #[test]
 fn remaining_capacity_and_incompatible_fluid_do_not_overfill_or_mix() {
  for (fluid,amount,expected) in [("x:copper",2155,2160),("x:copper",2160,2160),("minecraft:water",10,10)] {
   let (mut g,p,s,t,key,capacity)=fixture(1);
   let mut state=melter(&g,t); assert!(state.tank.insert(fluid,amount));
   let raw=store::read(&g.world,t,"tconstruct:melter").unwrap();
   g.world.tags.insert((t,"tconstruct:melter".into()),tconstruct_source_faucet_tank::patch_melter_tank(&raw,&state.tank).unwrap());
   assert_eq!(melter_lantern_transfer(&mut g,p,s,t,Faucet::new(),true),Some(true));
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
  assert_eq!(melter_lantern_transfer(&mut g,p,s,t,Faucet::new(),true),Some(true));
  assert_eq!(melter(&g,t).tank.tag,Some(first.clone())); assert_eq!(faucet(&g,p).pending.tag,Some(first));
  let mut state=melter(&g,t); state.tank.tag=Some(second);
  let raw=store::read(&g.world,t,"tconstruct:melter").unwrap();
  g.world.tags.insert((t,"tconstruct:melter".into()),tconstruct_source_faucet_tank::patch_melter_tank(&raw,&state.tank).unwrap());
  let f=faucet(&g,p); assert_eq!(melter_lantern_transfer(&mut g,p,s,t,f,false),Some(true));
  assert_eq!(melter(&g,t).tank.amount,10); assert_eq!(faucet(&g,p).pending.amount,80);
  for tag in [Tag::compound([("Potion",Tag::String("minecraft:swiftness".into())),("CustomPotionColor",Tag::Int(7))]),Tag::compound([("Potion",Tag::String("unknown:custom".into()))])] {
   let (mut g,p,s,t,key,capacity)=fixture(1); let mut input=Tank::new(capacity);
   assert!(input.insert_tagged("tconstruct:potion",Some(&tag),180)); g.world.tags.insert((s,key.into()),input.to_nbt());
   let before=g.world.tags.clone(); assert_eq!(melter_lantern_transfer(&mut g,p,s,t,Faucet::new(),true),Some(false)); assert_eq!(g.world.tags,before);
  }
 }
 #[test]
 fn reserved_packet_finishes_after_removed_source_and_stop_prevents_refill() {
  for removed in [false,true] {
   let (mut g,p,s,t,key,capacity)=fixture(1);
   assert_eq!(melter_lantern_transfer(&mut g,p,s,t,Faucet::new(),true),Some(true));
   if removed { g.world.blocks.remove(&s); g.world.block_entities.remove(&s); }
   else {
    let mut input=Tank::new(capacity); let tag=Tag::compound([("unknown",Tag::Int(7))]);
    assert!(input.insert_tagged("x:copper",Some(&tag),90)); g.world.tags.insert((s,key.into()),input.to_nbt());
    let f=faucet(&g,p); assert!(tconstruct_source_faucet_tank::stop(&mut g,p,f));
   }
   for _ in 0..9 { let f=faucet(&g,p); assert_eq!(melter_lantern_transfer(&mut g,p,s,t,f,false),Some(true)); }
   assert_eq!(melter(&g,t).tank.amount,90); assert_eq!(faucet(&g,p).pending.amount,0); assert!(!faucet(&g,p).active);
  }
 }
 #[test]
 fn formed_drains_preserve_both_machines_and_unselected_layers() {
  for controller_id in [12,13] { for drain_id in [11,14] {
   let (mut g,p,s,t,c)=drain_fixture(controller_id,drain_id);
   let source_before=store::read(&g.world,c,"tconstruct:smeltery").unwrap();
   let target_before=store::read(&g.world,t,"tconstruct:melter").unwrap();
   assert_eq!(melter_lantern_transfer(&mut g,p,s,t,Faucet::new(),true),Some(true));
   let raw=store::read(&g.world,c,"tconstruct:smeltery").unwrap();
   let state=Smeltery::from_nbt(&raw).unwrap();
   assert_eq!(state.tank.amount("x:copper"),90); assert_eq!(state.tank.amount("minecraft:water"),50);
   for field in ["slots","progress","fuel","temperature","controller_extension"] { assert_eq!(raw.get(field),source_before.get(field)); }
   assert_eq!(store::read(&g.world,t,"tconstruct:melter").unwrap().get("slots"),target_before.get("slots"));
   assert_eq!((melter(&g,t).tank.amount,faucet(&g,p).pending.amount),(10,80));
   assert_eq!(g.notifications,vec![c,t]);
   for _ in 0..8 { let f=faucet(&g,p); assert_eq!(melter_lantern_transfer(&mut g,p,s,t,f,false),Some(true)); }
   let f=faucet(&g,p); assert_eq!(melter_lantern_transfer(&mut g,p,s,t,f,false),Some(true));
   assert_eq!((melter(&g,t).tank.amount,faucet(&g,p).pending.amount),(100,80));
   let state=Smeltery::from_nbt(&store::read(&g.world,c,"tconstruct:smeltery").unwrap()).unwrap();
   assert_eq!(state.tank.amount("minecraft:water"),50); assert_eq!(state.tank.amount("x:copper"),0);
  } }
 }
 #[test]
 fn drain_failures_owner_structure_unknown_data_and_ducts_never_fall_through() {
  for failure in 0..8 {
   let (mut g,p,s,t,c)=drain_fixture(12,11);
   match failure {
    0=>g.world.fail=true,
    1=>g.world.formed=false,
    2=>{ g.world.chunks.owners.remove(&s); },
    3=>{ g.world.block_entities.remove(&c); },
    4=>{ g.world.blocks.remove(&c); },
    5=>{ g.world.chunks.owners.insert(s,t); },
    6=>{ if let Tag::Compound(raw)=g.world.tags.get_mut(&(c,"tconstruct:smeltery".into())).unwrap() { if let Some(Tag::Compound(tank))=raw.get_mut(&"tank".into()) { tank.insert("opaque".into(),Tag::Int(7)); } } },
    _=>{ g.world.blocks.insert(s,15); },
   }
   let before=g.world.tags.clone(); assert_eq!(melter_lantern_transfer(&mut g,p,s,t,Faucet::new(),true),Some(false));
   assert_eq!(g.world.tags,before); assert!(g.notifications.is_empty());
  }
 }
 #[test]
 fn drain_pending_potion_finishes_after_selection_changes_then_refuses_incompatible_refill() {
  let (mut g,p,s,t,c)=drain_fixture(13,14);
  let first=Tag::compound([("Potion",Tag::String("minecraft:swiftness".into()))]);
  let second=Tag::compound([("Potion",Tag::String("minecraft:healing".into()))]);
  let raw=store::read(&g.world,c,"tconstruct:smeltery").unwrap();
  let mut state=Smeltery::from_nbt(&raw).unwrap(); state.tank.fluids.clear();
  assert!(state.tank.insert_tagged("tconstruct:potion",Some(&first),90));
  assert!(state.tank.insert_tagged("tconstruct:potion",Some(&second),180));
  g.world.tags.insert((c,"tconstruct:smeltery".into()),patch_smeltery_controller_tank(&raw,&state).unwrap());
  assert_eq!(melter_lantern_transfer(&mut g,p,s,t,Faucet::new(),true),Some(true));
  assert_eq!(faucet(&g,p).pending.tag,Some(first.clone()));
  for _ in 0..9 { let f=faucet(&g,p); assert_eq!(melter_lantern_transfer(&mut g,p,s,t,f,false),Some(true)); }
  assert_eq!(melter(&g,t).tank.tag,Some(first)); assert_eq!(melter(&g,t).tank.amount,90);
  assert!(!faucet(&g,p).active);
  let state=Smeltery::from_nbt(&store::read(&g.world,c,"tconstruct:smeltery").unwrap()).unwrap();
  assert_eq!(state.tank.amount_tagged("tconstruct:potion",Some(&second)),180);
 }
 #[test]
 fn casting_tank_reservation_processes_container_once_and_commits_live_slots() {
  let (mut g,p,s,t)=portable_fixture();
  let target_before=store::read(&g.world,t,"tconstruct:melter").unwrap();
  assert_eq!(melter_lantern_transfer(&mut g,p,s,t,Faucet::new(),true),Some(true));
  let input=casting_tank_at(&g,s).unwrap();
  assert_eq!((input.tank.amount,melter(&g,t).tank.amount,faucet(&g,p).pending.amount),(1000,10,80));
  assert!(input.slots[0].is_none()); assert_eq!(input.slots[1].as_ref().unwrap().item.as_ref(),"minecraft:bucket");
  assert_eq!(store::read(&g.world,t,"tconstruct:melter").unwrap().get("slots"),target_before.get("slots"));
  assert_eq!((g.process_calls,g.signals),(1,1));
  let f=faucet(&g,p); assert_eq!(melter_lantern_transfer(&mut g,p,s,t,f,false),Some(true));
  assert_eq!(g.process_calls,1); assert_eq!(melter(&g,t).tank.amount,20);
  for _ in 0..7 { let f=faucet(&g,p); assert_eq!(melter_lantern_transfer(&mut g,p,s,t,f,false),Some(true)); }
  let f=faucet(&g,p); assert_eq!(melter_lantern_transfer(&mut g,p,s,t,f,false),Some(true));
  assert_eq!((casting_tank_at(&g,s).unwrap().tank.amount,melter(&g,t).tank.amount,faucet(&g,p).pending.amount),(910,100,80));
  assert_eq!(g.process_calls,2);
 }
 #[test]
 fn casting_tank_save_failure_and_reprocessing_races_do_not_commit_fluid() {
  for fault in 0..5 {
   let (mut g,p,s,t)=portable_fixture(); g.process_fault=fault; g.world.fail=fault==0;
   let before=g.world.tags.clone();
   assert_eq!(melter_lantern_transfer(&mut g,p,s,t,Faucet::new(),true),Some(false));
   assert_eq!(g.world.tags,before); assert_eq!(g.process_calls,1); assert!(g.notifications.is_empty()); assert_eq!(g.signals,0);
   if fault != 1 { let BlockEntity::Chest(slots)=g.world.block_entities.get(&s).unwrap(); assert_eq!(slots[0].as_ref().unwrap().item.as_ref(),"minecraft:water_bucket"); assert!(slots[1].is_none()); }
  }
 }
 #[test]
 fn casting_tank_opaque_or_stale_live_slots_and_full_melter_wait() {
  for failure in 0..4 {
   let (mut g,p,s,t)=portable_fixture();
   match failure {
    0=>{ if let Tag::Compound(raw)=g.world.tags.get_mut(&(s,"tconstruct:casting_tank".into())).unwrap() { raw.insert("opaque".into(),Tag::Int(7)); } },
    1=>{ let BlockEntity::Chest(slots)=g.world.block_entities.get_mut(&s).unwrap(); slots[0]=None; },
    2=>{ g.world.block_entities.remove(&s); },
    _=>{ let mut state=melter(&g,t); assert!(state.tank.insert("minecraft:water",2160)); let raw=store::read(&g.world,t,"tconstruct:melter").unwrap(); g.world.tags.insert((t,"tconstruct:melter".into()),tconstruct_source_faucet_tank::patch_melter_tank(&raw,&state.tank).unwrap()); },
   }
   let before=g.world.tags.clone(); assert_eq!(melter_lantern_transfer(&mut g,p,s,t,Faucet::new(),true),Some(failure==3));
   assert_eq!(g.world.tags,before); assert_eq!(g.process_calls,0); assert!(g.notifications.is_empty());
  }
 }
 #[test]
 fn casting_tank_base_potion_packet_preserves_tag_and_stops_before_refilled_water() {
  let (mut g,p,s,t)=portable_fixture();
  let mut input=casting_tank_at(&g,s).unwrap(); input.tank=Tank::new(4000);
  let tag=Tag::compound([("Potion",Tag::String("minecraft:healing".into()))]);
  assert!(input.tank.insert_tagged("tconstruct:potion",Some(&tag),90));
  g.world.tags.insert((s,"tconstruct:casting_tank".into()),input.to_nbt());
  assert_eq!(melter_lantern_transfer(&mut g,p,s,t,Faucet::new(),true),Some(true));
  assert_eq!(faucet(&g,p).pending.tag,Some(tag.clone()));
  assert_eq!(casting_tank_at(&g,s).unwrap().tank.fluid.as_deref(),Some("minecraft:water"));
  for _ in 0..9 { let f=faucet(&g,p); assert_eq!(melter_lantern_transfer(&mut g,p,s,t,f,false),Some(true)); }
  assert_eq!((melter(&g,t).tank.amount,casting_tank_at(&g,s).unwrap().tank.amount),(90,1000));
  assert_eq!(melter(&g,t).tank.tag,Some(tag)); assert!(!faucet(&g,p).active); assert_eq!(g.process_calls,1);
 }
 #[test]
 fn melter_chain_preserves_both_raw_machine_states_and_live_inventories() {
  let (mut g,p,s,t)=melter_source_fixture();
  let source_before=store::read(&g.world,s,"tconstruct:melter").unwrap();
  let target_before=store::read(&g.world,t,"tconstruct:melter").unwrap();
  assert_eq!(melter_lantern_transfer(&mut g,p,s,t,Faucet::new(),true),Some(true));
  for (at,before) in [(s,source_before),(t,target_before)] {
   let after=store::read(&g.world,at,"tconstruct:melter").unwrap();
   for field in ["slots","progress","phase","fuel","fuel_quality","temperature","rate","heat_codes"] { assert_eq!(after.get(field),before.get(field)); }
   let BlockEntity::Chest(slots)=g.world.block_entities.get(&at).unwrap();
   assert_eq!(slots[0].as_ref().unwrap().item.as_ref(),"minecraft:iron_ore");
  }
  assert_eq!((melter(&g,s).tank.amount,melter(&g,t).tank.amount,faucet(&g,p).pending.amount),(90,10,80));
  assert_eq!(g.notifications,vec![s,t]); assert_eq!(g.process_calls,0);
  for _ in 0..8 { let f=faucet(&g,p); assert_eq!(melter_lantern_transfer(&mut g,p,s,t,f,false),Some(true)); }
  let f=faucet(&g,p); assert_eq!(melter_lantern_transfer(&mut g,p,s,t,f,false),Some(true));
  assert_eq!((melter(&g,s).tank.amount,melter(&g,t).tank.amount,faucet(&g,p).pending.amount),(0,100,80));
 }
 #[test]
 fn melter_chain_refuses_unknown_source_tank_failed_save_missing_entity_and_aliases() {
  for failure in 0..5 {
   let (mut g,p,s,t)=melter_source_fixture();
   match failure {
    0=>g.world.fail=true,
    1=>{ g.world.block_entities.remove(&s); },
    2=>{ g.world.tags.remove(&(s,"tconstruct:melter".into())); },
    3=>{ if let Tag::Compound(raw)=g.world.tags.get_mut(&(s,"tconstruct:melter".into())).unwrap() { if let Some(Tag::Compound(tank))=raw.get_mut(&"tank".into()) { tank.insert("opaque".into(),Tag::Int(7)); } } },
    _=>{ if let Tag::Compound(raw)=g.world.tags.get_mut(&(s,"tconstruct:melter".into())).unwrap() { raw.insert("slots".into(),Tag::List(vec![])); } },
   }
   let before=g.world.tags.clone(); assert_eq!(melter_lantern_transfer(&mut g,p,s,t,Faucet::new(),true),Some(false));
   assert_eq!(g.world.tags,before); assert!(g.notifications.is_empty());
  }
  let (mut g,p,s,t)=melter_source_fixture(); let before=g.world.tags.clone();
  assert_eq!(melter_lantern_transfer(&mut g,p,s,s,Faucet::new(),true),Some(false));
  assert_eq!(melter_lantern_transfer(&mut g,s,s,t,Faucet::new(),true),Some(false)); assert_eq!(g.world.tags,before);
 }
 #[test]
 fn melter_chain_potion_packet_outlives_refill_and_capacity_is_bounded() {
  let (mut g,p,s,t)=melter_source_fixture();
  let first=Tag::compound([("Potion",Tag::String("minecraft:healing".into()))]);
  let second=Tag::compound([("Potion",Tag::String("minecraft:swiftness".into()))]);
  let mut input=Tank::new(2160); assert!(input.insert_tagged("tconstruct:potion",Some(&first),90));
  let raw=store::read(&g.world,s,"tconstruct:melter").unwrap();
  g.world.tags.insert((s,"tconstruct:melter".into()),tconstruct_source_faucet_tank::patch_melter_tank(&raw,&input).unwrap());
  assert_eq!(melter_lantern_transfer(&mut g,p,s,t,Faucet::new(),true),Some(true));
  let mut input=Tank::new(2160); assert!(input.insert_tagged("tconstruct:potion",Some(&second),180));
  let raw=store::read(&g.world,s,"tconstruct:melter").unwrap();
  g.world.tags.insert((s,"tconstruct:melter".into()),tconstruct_source_faucet_tank::patch_melter_tank(&raw,&input).unwrap());
  for _ in 0..9 { let f=faucet(&g,p); assert_eq!(melter_lantern_transfer(&mut g,p,s,t,f,false),Some(true)); }
  assert_eq!((melter(&g,s).tank.amount,melter(&g,t).tank.amount),(180,90));
  assert_eq!(melter(&g,s).tank.tag,Some(second)); assert_eq!(melter(&g,t).tank.tag,Some(first)); assert!(!faucet(&g,p).active);
  let (mut g,p,s,t)=melter_source_fixture(); let mut output=Tank::new(2160); assert!(output.insert("minecraft:water",2155));
  let raw=store::read(&g.world,t,"tconstruct:melter").unwrap();
  g.world.tags.insert((t,"tconstruct:melter".into()),tconstruct_source_faucet_tank::patch_melter_tank(&raw,&output).unwrap());
  assert_eq!(melter_lantern_transfer(&mut g,p,s,t,Faucet::new(),true),Some(true));
  assert_eq!((melter(&g,s).tank.amount,melter(&g,t).tank.amount,faucet(&g,p).pending.amount),(175,2160,0));
 }
 #[test]
 fn both_lanterns_accept_each_bounded_source_without_exceeding_fifty_mb() {
  for lantern_id in [17,18] { for source_id in 1..=12 {
   let (mut g,p,s,t)=lantern_fixture(lantern_id,source_id);
   let original_slots={ let BlockEntity::Chest(slots)=g.world.block_entities.get(&t).unwrap(); slots[0].as_ref().unwrap().item.clone() };
   assert_eq!(melter_lantern_transfer(&mut g,p,s,t,Faucet::new(),true),Some(true));
   assert_eq!((lantern(&g,t).amount,faucet(&g,p).pending.amount),(10,40));
   for _ in 0..4 { let f=faucet(&g,p); assert_eq!(melter_lantern_transfer(&mut g,p,s,t,f,false),Some(true)); }
   assert_eq!((lantern(&g,t).amount,faucet(&g,p).pending.amount),(50,0));
   let f=faucet(&g,p); assert_eq!(melter_lantern_transfer(&mut g,p,s,t,f,false),Some(true)); assert!(!faucet(&g,p).active);
   let BlockEntity::Chest(slots)=g.world.block_entities.get(&t).unwrap(); assert_eq!(slots[0].as_ref().unwrap().item,original_slots);
  } }
 }
 #[test]
 fn lantern_saved_state_failures_and_reprocessing_races_do_not_drain() {
  for failure in 0..6 {
   let (mut g,p,s,t)=lantern_fixture(17,12);
   match failure {
    0=>g.world.fail=true,
    1=>{ g.world.block_entities.remove(&t); },
    2=>{ g.world.tags.remove(&(t,"tconstruct:lantern".into())); },
    3=>{ if let Tag::Compound(raw)=g.world.tags.get_mut(&(t,"tconstruct:lantern".into())).unwrap() { raw.insert("opaque".into(),Tag::Int(7)); } },
    4=>g.process_fault=2,
    _=>g.process_fault=4,
   }
   let before=g.world.tags.clone(); assert_eq!(melter_lantern_transfer(&mut g,p,s,t,Faucet::new(),true),Some(false));
   assert_eq!(g.world.tags,before); assert!(g.notifications.is_empty());
   let BlockEntity::Chest(slots)=g.world.block_entities.get(&s).unwrap(); assert_eq!(slots[0].as_ref().unwrap().item.as_ref(),"minecraft:water_bucket");
  }
 }
 #[test]
 fn lantern_exact_potion_identity_remaining_capacity_and_unsupported_tags() {
  let first=Tag::compound([("Potion",Tag::String("minecraft:healing".into()))]);
  let second=Tag::compound([("Potion",Tag::String("minecraft:swiftness".into()))]);
  for matching in [false,true] {
   let (mut g,p,s,t)=lantern_fixture(18,1); let mut input=Tank::new(4000);
   assert!(input.insert_tagged("tconstruct:potion",Some(&first),90)); g.world.tags.insert((s,"tconstruct:fuel_tank".into()),input.to_nbt());
   let mut output=Tank::new(50); assert!(output.insert_tagged("tconstruct:potion",Some(if matching {&first} else {&second}),45));
   g.world.tags.insert((t,"tconstruct:lantern".into()),output.to_nbt());
   assert_eq!(melter_lantern_transfer(&mut g,p,s,t,Faucet::new(),true),Some(true));
   assert_eq!(lantern(&g,t).amount,if matching {50} else {45});
   assert_eq!(Tank::from_nbt(&store::read(&g.world,s,"tconstruct:fuel_tank").unwrap(),4000).unwrap().amount,if matching {85} else {90});
  }
  let (mut g,p,s,t)=lantern_fixture(17,1); let mut input=Tank::new(4000);
  let unsupported=Tag::compound([("Potion",Tag::String("minecraft:healing".into())),("CustomPotionColor",Tag::Int(7))]);
  assert!(input.insert_tagged("tconstruct:potion",Some(&unsupported),90)); g.world.tags.insert((s,"tconstruct:fuel_tank".into()),input.to_nbt());
  let before=g.world.tags.clone(); assert_eq!(melter_lantern_transfer(&mut g,p,s,t,Faucet::new(),true),Some(false)); assert_eq!(g.world.tags,before);
 }
 #[test]
 fn both_lantern_sources_fill_melter_and_each_lantern_without_losing_the_packet() {
  for source_id in [17,18] { for target_id in [9,17,18] {
   let (mut g,p,s,t)=lantern_source_fixture(source_id,target_id);
   assert_eq!(melter_lantern_transfer(&mut g,p,s,t,Faucet::new(),true),Some(true));
   assert_eq!((lantern(&g,s).amount,faucet(&g,p).pending.amount),(0,40));
   assert!(lantern(&g,s).fluid.is_none()); assert!(lantern(&g,s).tag.is_none());
   let f=faucet(&g,p); assert!(tconstruct_source_faucet_tank::stop(&mut g,p,f));
   for _ in 0..5 { let f=faucet(&g,p); assert_eq!(melter_lantern_transfer(&mut g,p,s,t,f,false),Some(true)); }
   assert_eq!(if target_id==9 {melter(&g,t).tank.amount} else {lantern(&g,t).amount},50);
   assert_eq!(faucet(&g,p).pending.amount,0); assert!(!faucet(&g,p).active);
   assert_eq!(g.process_calls,0);
  } }
 }
 #[test]
 fn lantern_source_unknown_state_missing_entity_failed_save_and_aliases_wait() {
  for failure in 0..5 {
   let (mut g,p,s,t)=lantern_source_fixture(17,9);
   match failure {
    0=>g.world.fail=true,
    1=>{ g.world.block_entities.remove(&s); },
    2=>{ g.world.tags.remove(&(s,"tconstruct:lantern".into())); },
    3=>{ if let Tag::Compound(raw)=g.world.tags.get_mut(&(s,"tconstruct:lantern".into())).unwrap() { raw.insert("opaque".into(),Tag::Int(7)); } },
    _=>{ g.world.chunks.loaded.remove(&(-1,0)); },
   }
   let before=g.world.tags.clone(); assert_eq!(melter_lantern_transfer(&mut g,p,s,t,Faucet::new(),true),Some(false));
   assert_eq!(g.world.tags,before); assert!(g.notifications.is_empty());
  }
  let (mut g,p,s,t)=lantern_source_fixture(18,17); let before=g.world.tags.clone();
  assert_eq!(melter_lantern_transfer(&mut g,p,s,s,Faucet::new(),true),Some(false));
  assert_eq!(melter_lantern_transfer(&mut g,s,s,t,Faucet::new(),true),Some(false)); assert_eq!(g.world.tags,before);
 }
 #[test]
 fn lantern_potion_packet_survives_refill_but_opaque_source_waits() {
  let first=Tag::compound([("Potion",Tag::String("minecraft:healing".into()))]);
  let second=Tag::compound([("Potion",Tag::String("minecraft:swiftness".into()))]);
  let (mut g,p,s,t)=lantern_source_fixture(18,9); let mut tank=Tank::new(50);
  assert!(tank.insert_tagged("tconstruct:potion",Some(&first),50)); g.world.tags.insert((s,"tconstruct:lantern".into()),tank.to_nbt());
  assert_eq!(melter_lantern_transfer(&mut g,p,s,t,Faucet::new(),true),Some(true));
  let mut tank=Tank::new(50); assert!(tank.insert_tagged("tconstruct:potion",Some(&second),50));
  let valid=tank.to_nbt(); let mut opaque=valid.clone(); if let Tag::Compound(fields)=&mut opaque {fields.insert("opaque".into(),Tag::Int(7));}
  g.world.tags.insert((s,"tconstruct:lantern".into()),opaque);
  let f=faucet(&g,p); let before=g.world.tags.clone(); assert_eq!(melter_lantern_transfer(&mut g,p,s,t,f,false),Some(false)); assert_eq!(g.world.tags,before);
  g.world.tags.insert((s,"tconstruct:lantern".into()),valid);
  for _ in 0..5 { let f=faucet(&g,p); assert_eq!(melter_lantern_transfer(&mut g,p,s,t,f,false),Some(true)); }
  assert_eq!((melter(&g,t).tank.amount,lantern(&g,s).amount),(50,50));
  assert_eq!(melter(&g,t).tank.tag,Some(first)); assert_eq!(lantern(&g,s).tag,Some(second)); assert!(!faucet(&g,p).active);
 }
 #[test]
 fn both_lanterns_feed_all_eight_storage_tanks_and_gauges() {
  for source_id in [17,18] { for target_id in 1..=8 {
   let (mut g,p,s,t,key,capacity)=lantern_storage_fixture(source_id,target_id);
   assert_eq!(melter_lantern_transfer(&mut g,p,s,t,Faucet::new(),true),Some(true));
   assert_eq!((lantern(&g,s).amount,storage(&g,t,key,capacity).amount,faucet(&g,p).pending.amount),(0,10,40));
   for _ in 0..5 { let f=faucet(&g,p); assert_eq!(melter_lantern_transfer(&mut g,p,s,t,f,false),Some(true)); }
   assert_eq!(storage(&g,t,key,capacity).amount,50); assert!(!faucet(&g,p).active);
  } }
  let (mut g,p,s,t,_,_)=fixture(1); g.world.blocks.insert(t,1);
  assert_eq!(melter_lantern_transfer(&mut g,p,s,t,Faucet::new(),true),None);
 }
 #[test]
 fn lantern_storage_destination_failures_wait_without_erasing_source() {
  for failure in 0..5 {
   let (mut g,p,s,t,key,_)=lantern_storage_fixture(18,5);
   match failure {
    0=>g.world.fail=true,
    1=>{ g.world.block_entities.remove(&t); },
    2=>{ g.world.tags.remove(&(t,key.into())); },
    3=>{ if let Tag::Compound(raw)=g.world.tags.get_mut(&(t,key.into())).unwrap() {raw.insert("opaque".into(),Tag::Int(7));} },
    _=>{ g.world.chunks.loaded.remove(&(0,0)); },
   }
   let before=g.world.tags.clone(); assert_eq!(melter_lantern_transfer(&mut g,p,s,t,Faucet::new(),true),Some(false));
   assert_eq!(g.world.tags,before); assert!(g.notifications.is_empty());
  }
 }
 #[test]
 fn lantern_storage_capacity_and_potion_identity_remain_exact() {
  let first=Tag::compound([("Potion",Tag::String("minecraft:healing".into()))]);
  let second=Tag::compound([("Potion",Tag::String("minecraft:swiftness".into()))]);
  for target_id in [1,5] { for matching in [false,true] {
   let (mut g,p,s,t,key,capacity)=lantern_storage_fixture(17,target_id);
   let mut input=Tank::new(50); assert!(input.insert_tagged("tconstruct:potion",Some(&first),50)); g.world.tags.insert((s,"tconstruct:lantern".into()),input.to_nbt());
   let mut output=Tank::new(capacity); assert!(output.insert_tagged("tconstruct:potion",Some(if matching {&first} else {&second}),capacity-5)); g.world.tags.insert((t,key.into()),output.to_nbt());
   assert_eq!(melter_lantern_transfer(&mut g,p,s,t,Faucet::new(),true),Some(true));
   assert_eq!(storage(&g,t,key,capacity).amount,if matching {capacity} else {capacity-5});
   assert_eq!(lantern(&g,s).amount,if matching {45} else {50}); assert_eq!(faucet(&g,p).pending.amount,0);
  } }
 }
 #[test]
 fn cannon_output_all_eleven_targets_keep_saved_and_live_items() {
  for target_id in [1,2,3,4,5,6,7,8,9,17,18] {
   let (mut g,p,s,t,key,capacity)=cannon_output_fixture(target_id);
   let before=store::read(&g.world,s,"tconstruct:fluid_cannon").unwrap();
   let target_before=store::read(&g.world,t,key).unwrap();
   let BlockEntity::Chest(slots)=g.world.block_entities.get(&s).unwrap(); let live_before=slots.clone();
   let BlockEntity::Chest(slots)=g.world.block_entities.get(&t).unwrap(); let target_live=slots.clone();
   assert_eq!(melter_lantern_transfer(&mut g,p,s,t,Faucet::new(),true),Some(true));
   let input=cannon_at(&g,s).unwrap();
   let reserved=90.min(capacity);
   assert_eq!((input.tank.amount,cannon_output(&g,t,key,capacity).amount,faucet(&g,p).pending.amount),(180-reserved,10,reserved-10));
   assert_eq!(store::read(&g.world,s,"tconstruct:fluid_cannon"),super::tconstruct_source_faucet_tank::patch_melter_tank(&before,&input.tank));
   if target_id==9 {
    for field in ["slots","progress","phase","fuel","fuel_quality","temperature","rate","heat_codes"] {
     assert_eq!(store::read(&g.world,t,key).unwrap().get(field),target_before.get(field));
    }
   }
   for _ in 0..9 {let f=faucet(&g,p);assert_eq!(melter_lantern_transfer(&mut g,p,s,t,f,false),Some(true));}
   let input=cannon_at(&g,s).unwrap();
   if capacity==50 { assert_eq!((input.tank.amount,cannon_output(&g,t,key,capacity).amount),(130,50)); }
   else { assert_eq!((input.tank.amount,cannon_output(&g,t,key,capacity).amount,faucet(&g,p).pending.amount),(0,100,80)); }
   let BlockEntity::Chest(slots)=g.world.block_entities.get(&s).unwrap();assert_eq!(*slots,live_before);
   let BlockEntity::Chest(slots)=g.world.block_entities.get(&t).unwrap();assert_eq!(*slots,target_live);
   assert_eq!(g.signals,if capacity==50 {1}else{2});assert_eq!(g.process_calls,0);
  }
 }
 #[test]
 fn cannon_output_refuses_bad_source_and_failed_saves() {
  for target_id in [1,5,9,17] { for failure in 0..7 {
   let (mut g,p,s,t,_,_)=cannon_output_fixture(target_id);
   match failure {
    0=>g.world.fail=true,
    1=>{g.world.block_entities.remove(&s);},
    2=>{g.world.tags.remove(&(s,"tconstruct:fluid_cannon".into()));},
    3=>{if let Tag::Compound(raw)=g.world.tags.get_mut(&(s,"tconstruct:fluid_cannon".into())).unwrap(){raw.insert("future".into(),Tag::Int(1));}},
    4=>{let BlockEntity::Chest(slots)=g.world.block_entities.get_mut(&s).unwrap();slots[0]=None;},
    5=>{g.world.block_entities.insert(s,BlockEntity::Chest(vec![]));},
    _=>{g.world.chunks.loaded.remove(&(-1,0));},
   }
   let before=g.world.tags.clone();assert_eq!(melter_lantern_transfer(&mut g,p,s,t,Faucet::new(),true),Some(false));
   assert_eq!(g.world.tags,before);assert!(g.notifications.is_empty());assert_eq!(g.signals,0);
  }}
 }
 #[test]
 fn cannon_output_last_five_mb_matches_all_destination_capacities() {
  for target_id in [1,5,9,17,18] {
   let (mut g,p,s,t,key,capacity)=cannon_output_fixture(target_id);
   let mut output=Tank::new(capacity);assert!(output.insert("minecraft:water",capacity-5));
   let raw=store::read(&g.world,t,key).unwrap();
   let next=if target_id==9 {super::tconstruct_source_faucet_tank::patch_melter_tank(&raw,&output).unwrap()}else{output.to_nbt()};
   g.world.tags.insert((t,key.into()),next);
   assert_eq!(melter_lantern_transfer(&mut g,p,s,t,Faucet::new(),true),Some(true));
   assert_eq!((cannon_at(&g,s).unwrap().tank.amount,cannon_output(&g,t,key,capacity).amount,faucet(&g,p).pending.amount),(175,capacity,0));
  }
 }
 #[test]
 fn cannon_output_reserved_packet_survives_potion_refill_and_custom_tags_wait() {
  for target_id in [1,9,17] {
   let (mut g,p,s,t,key,capacity)=cannon_output_fixture(target_id);
   let mut input=cannon_at(&g,s).unwrap();input.tank=Tank::new(2000);assert!(input.tank.insert("minecraft:water",50));
   g.world.tags.insert((s,"tconstruct:fluid_cannon".into()),input.to_nbt());
   assert_eq!(melter_lantern_transfer(&mut g,p,s,t,Faucet::new(),true),Some(true));
   let mut input=cannon_at(&g,s).unwrap();let tag=Tag::compound([("Potion",Tag::String("minecraft:healing".into()))]);
   assert!(input.tank.insert_tagged("tconstruct:potion",Some(&tag),250));g.world.tags.insert((s,"tconstruct:fluid_cannon".into()),input.to_nbt());
   for _ in 0..5 {let f=faucet(&g,p);assert_eq!(melter_lantern_transfer(&mut g,p,s,t,f,false),Some(true));}
   assert_eq!(cannon_output(&g,t,key,capacity).amount,50);assert_eq!(cannon_at(&g,s).unwrap().tank.tag,Some(tag));
   assert!(!faucet(&g,p).active);assert_eq!(cannon_at(&g,s).unwrap().tank.amount,250);
   let empty=Tank::new(capacity);let raw=store::read(&g.world,t,key).unwrap();
   let next=if target_id==9 {super::tconstruct_source_faucet_tank::patch_melter_tank(&raw,&empty).unwrap()}else{empty.to_nbt()};
   g.world.tags.insert((t,key.into()),next);
   let mut input=cannon_at(&g,s).unwrap();if let Some(Tag::Compound(fields))=&mut input.tank.tag {fields.insert("CustomPotionColor".into(),Tag::Int(1));}
   g.world.tags.insert((s,"tconstruct:fluid_cannon".into()),input.to_nbt());
   let f=faucet(&g,p);let before=g.world.tags.clone();assert_eq!(melter_lantern_transfer(&mut g,p,s,t,f,true),Some(false));assert_eq!(g.world.tags,before);
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
 let world=World{blocks:HashMap::from([(s,id),(t,9),(p,10)]),block_entities:HashMap::from([(s,containers::BlockEntity::Chest(vec![])),(t,containers::BlockEntity::Chest(vec![Some(horizon_items::items::ItemStack::new("minecraft:iron_ore",1)),None,None]))]),chunks:Chunks{loaded:HashMap::from([((-1,0),()),((0,0),())]),owners:HashMap::new()},tags,fail:false,formed:true};
 (Game{res:Res,world,notifications:vec![],process_calls:0,process_fault:0,signals:0,_life:std::marker::PhantomData},p,s,t,key,capacity)
}

fn drain_fixture(controller_id:usize,drain_id:usize)->(Game<'static>,Pos,Pos,Pos,Pos) {
 let (mut g,p,s,t,key,_)=fixture(1); let c=(-3,1,0);
 g.world.blocks.insert(s,drain_id); g.world.blocks.insert(c,controller_id);
 g.world.block_entities.remove(&s); g.world.tags.remove(&(s,key.into()));
 g.world.block_entities.insert(c,containers::BlockEntity::Chest(vec![None]));
 g.world.chunks.owners.insert(s,c);
 let mut state=Smeltery::new(); state.resize(1,4000); state.fuel=87; state.temperature=1000; state.progress[0]=-1;
 state.slots[0]=Some(horizon_items::items::ItemStack::new("minecraft:copper_ore",1));
 assert!(state.tank.insert("x:copper",180)); assert!(state.tank.insert("minecraft:water",50));
 let mut raw=state.to_nbt(); if let Tag::Compound(fields)=&mut raw {
  fields.insert("controller_extension".into(),Tag::Int(123));
  if let Some(Tag::List(slots))=fields.get_mut(&"slots".into()) { if let Tag::Compound(slot)=&mut slots[0] { slot.insert("opaque".into(),Tag::Int(7)); } }
 }
 g.world.tags.insert((c,"tconstruct:smeltery".into()),raw);
 (g,p,s,t,c)
}

fn portable_fixture()->(Game<'static>,Pos,Pos,Pos) {
 let (mut g,p,s,t,key,_)=fixture(1); g.world.blocks.insert(s,16); g.world.tags.remove(&(s,key.into()));
 let mut state=tconstruct_source_casting_tank::CastingTank::new(); assert!(state.tank.insert("minecraft:water",90));
 state.slots[0]=Some(horizon_items::items::ItemStack::new("minecraft:water_bucket",1));
 g.world.block_entities.insert(s,containers::BlockEntity::Chest(state.slots.to_vec()));
 g.world.tags.insert((s,"tconstruct:casting_tank".into()),state.to_nbt());
 (g,p,s,t)
}

fn melter_source_fixture()->(Game<'static>,Pos,Pos,Pos) {
 let (mut g,p,s,t,key,_)=fixture(1); g.world.blocks.insert(s,9); g.world.tags.remove(&(s,key.into()));
 let raw=g.world.tags.get(&(t,"tconstruct:melter".into())).unwrap().clone();
 let mut tank=Tank::new(2160); assert!(tank.insert("minecraft:water",180));
 let raw=tconstruct_source_faucet_tank::patch_melter_tank(&raw,&tank).unwrap();
 g.world.tags.insert((s,"tconstruct:melter".into()),raw);
 g.world.block_entities.insert(s,containers::BlockEntity::Chest(vec![Some(horizon_items::items::ItemStack::new("minecraft:iron_ore",1)),None,None]));
 (g,p,s,t)
}

fn lantern(g:&Game<'_>,at:Pos)->Tank { Tank::from_nbt(g.world.tags.get(&(at,"tconstruct:lantern".into())).unwrap(),50).unwrap() }
fn lantern_fixture(lantern_id:usize,source_id:usize)->(Game<'static>,Pos,Pos,Pos) {
 let (mut g,p,s,t)=match source_id {
  1..=8=>{ let (g,p,s,t,_,_)=fixture(source_id); (g,p,s,t) },
  9=>{ let (g,p,s,t,_)=drain_fixture(12,11); (g,p,s,t) },
  10=>{ let (g,p,s,t,_)=drain_fixture(13,14); (g,p,s,t) },
  11=>melter_source_fixture(),
  _=>portable_fixture(),
 };
 g.world.blocks.insert(t,lantern_id); g.world.tags.remove(&(t,"tconstruct:melter".into()));
 g.world.tags.insert((t,"tconstruct:lantern".into()),Tank::new(50).to_nbt());
 (g,p,s,t)
}

fn lantern_source_fixture(source_id:usize,target_id:usize)->(Game<'static>,Pos,Pos,Pos) {
 let (mut g,p,s,t,key,_)=fixture(1); g.world.blocks.insert(s,source_id); g.world.tags.remove(&(s,key.into()));
 let mut input=Tank::new(50); assert!(input.insert("minecraft:water",50));
 g.world.tags.insert((s,"tconstruct:lantern".into()),input.to_nbt());
 if target_id!=9 { g.world.blocks.insert(t,target_id); g.world.tags.remove(&(t,"tconstruct:melter".into())); g.world.tags.insert((t,"tconstruct:lantern".into()),Tank::new(50).to_nbt()); }
 (g,p,s,t)
}

fn storage(g:&Game<'_>,at:Pos,key:&str,capacity:u32)->Tank { Tank::from_nbt(g.world.tags.get(&(at,key.into())).unwrap(),capacity).unwrap() }
fn lantern_storage_fixture(source_id:usize,target_id:usize)->(Game<'static>,Pos,Pos,Pos,&'static str,u32) {
 let (mut g,p,s,t)=lantern_source_fixture(source_id,9);
 g.world.blocks.insert(t,target_id); g.world.tags.remove(&(t,"tconstruct:melter".into()));
 let (key,capacity)=tconstruct_source_faucet_tank::tank_kind(&Facts.get(target_id).name).unwrap();
 g.world.tags.insert((t,key.into()),Tank::new(capacity).to_nbt());
 (g,p,s,t,key,capacity)
}

fn cannon_output_fixture(target_id:usize)->(Game<'static>,Pos,Pos,Pos,&'static str,u32) {
 let (mut g,p,s,t,key,_)=fixture(1);g.world.blocks.insert(s,19);g.world.tags.remove(&(s,key.into()));
 let mut input=tconstruct_source_fluid_cannon::Cannon::new();assert!(input.tank.insert("minecraft:water",180));
 let mut item=horizon_items::items::ItemStack::new("minecraft:stone",32);
 item.data_mut().set_other("minecraft:custom_data",Some(Tag::compound([("keep",Tag::Int(7))])));input.item=Some(item);
 g.world.block_entities.insert(s,containers::BlockEntity::Chest(vec![input.item.clone()]));
 g.world.tags.insert((s,"tconstruct:fluid_cannon".into()),input.to_nbt());
 let (target_key,capacity)=if target_id==9 {("tconstruct:melter",2160)}else{
  let (key,capacity)=if matches!(target_id,17|18){("tconstruct:lantern",50)}else{tconstruct_source_faucet_tank::tank_kind(&Facts.get(target_id).name).unwrap()};
  g.world.blocks.insert(t,target_id);g.world.tags.remove(&(t,"tconstruct:melter".into()));g.world.tags.insert((t,key.into()),Tank::new(capacity).to_nbt());(key,capacity)
 };
 (g,p,s,t,target_key,capacity)
}
fn cannon_output(g:&Game<'_>,at:Pos,key:&str,capacity:u32)->Tank {
 if key=="tconstruct:melter" {melter(g,at).tank}else{storage(g,at,key,capacity)}
}
