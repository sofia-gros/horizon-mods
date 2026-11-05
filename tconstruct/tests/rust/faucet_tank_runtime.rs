#[path = "../../mixins/tconstruct/source-melter-state.rs"] mod tconstruct_source_melter_state;
#[path = "../../mixins/tconstruct/source-faucet-tank.rs"] mod tconstruct_source_faucet_tank;
use std::collections::HashMap;
use crate::containers;
use horizon_world::nbt::Tag;
use tconstruct_source_melter_state::{Faucet,Tank};
type Pos=(i32,i32,i32);
struct Info { name:String }
struct Facts;
impl Facts { fn get(&self, id:usize)->Info { Info {name: match id {1=>"tconstruct:seared_melter",2=>"tconstruct:seared_fuel_tank",3=>"tconstruct:seared_ingot_tank",_=>"minecraft:air"}.into()} } }
struct Res { facts:Facts }
impl Res { fn facts(&self)->&Facts {&self.facts} }
struct World { blocks:HashMap<Pos,usize>, block_entities:HashMap<Pos,containers::BlockEntity>, tags:HashMap<(Pos,String),Tag>, fail:bool }
impl World { fn block(&self,x:i32,y:i32,z:i32)->usize {*self.blocks.get(&(x,y,z)).unwrap_or(&0)} }
struct Game<'a> { res:Res, world:World, _life:std::marker::PhantomData<&'a ()> }
mod tconstruct_source_smeltery_fuel { pub(super) fn capacity(name:&str)->Option<u32> { match name {"tconstruct:seared_fuel_tank"=>Some(4000),"tconstruct:seared_ingot_tank"=>Some(4320),_=>None} } }
mod tconstruct_source_machine_runtime { pub(super) fn kind(name:&str)->Option<&'static str> { match name {"tconstruct:seared_fuel_tank"=>Some("tconstruct:fuel_tank"),"tconstruct:seared_ingot_tank"=>Some("tconstruct:ingot_tank"),_=>None} } }
mod tconstruct_source_melter_store {
 use super::*;
 pub(super) fn read(w:&World,p:Pos,k:&str)->Option<Tag>{w.tags.get(&(p,k.into())).cloned()}
 pub(super) fn write_many(w:&mut World, updates:&[(Pos,&str,Tag)])->bool {
  if w.fail {return false} for (p,k,t) in updates {if !w.tags.contains_key(&(*p,(*k).into())) {return false}}
  for (p,k,t) in updates {w.tags.insert((*p,(*k).into()),t.clone());} true
 }
}
fn fixture()->(Game<'static>,Pos,Pos,Pos) {
 let (pos,source,target)=((0,1,0),(-1,1,0),(0,0,0)); let mut tank=Tank::new(4000); tank.insert("x:copper",180);
 let mut world=World{blocks:HashMap::from([(source,2),(target,3)]),block_entities:HashMap::from([(source,containers::BlockEntity::Chest(vec![])),(target,containers::BlockEntity::Chest(vec![]))]),tags:HashMap::new(),fail:false};
 world.tags.insert((source,"tconstruct:fuel_tank".into()),tank.to_nbt());
 world.tags.insert((target,"tconstruct:ingot_tank".into()),Tank::new(4320).to_nbt());
 world.tags.insert((pos,"tconstruct:faucet".into()),Faucet::new().to_nbt());
 (Game{res:Res{facts:Facts},world,_life:std::marker::PhantomData},pos,source,target)
}
#[test] fn activation_and_ticks_write_three_states() {
 let (mut game,pos,source,target)=fixture();
 assert_eq!(tconstruct_source_faucet_tank::activate(&mut game,pos,source,target,Faucet::new()),Some(true));
 let source_amount=Tank::from_nbt(&tconstruct_source_melter_store::read(&game.world,source,"tconstruct:fuel_tank").unwrap(),4000).unwrap().amount; assert_eq!(source_amount,90);
 let faucet=Faucet::from_nbt(&tconstruct_source_melter_store::read(&game.world,pos,"tconstruct:faucet").unwrap()).unwrap();
 assert_eq!(faucet.pending.amount,80); assert!(tconstruct_source_faucet_tank::tick(&mut game,pos,source,target,faucet));
 let out=Tank::from_nbt(&tconstruct_source_melter_store::read(&game.world,target,"tconstruct:ingot_tank").unwrap(),4320).unwrap(); assert_eq!(out.amount,20);
}
#[test] fn failed_write_and_unreadable_target_do_not_drain() {
 let (mut game,pos,source,target)=fixture(); let old=game.world.tags.clone(); game.world.fail=true;
 assert_eq!(tconstruct_source_faucet_tank::activate(&mut game,pos,source,target,Faucet::new()),Some(false)); assert_eq!(game.world.tags,old);
 game.world.fail=false; game.world.tags.remove(&(target,"tconstruct:ingot_tank".into())); let remaining=game.world.tags.clone();
 assert_eq!(tconstruct_source_faucet_tank::activate(&mut game,pos,source,target,Faucet::new()),Some(false)); assert_eq!(game.world.tags,remaining);
}
#[test] fn pending_packet_pours_after_source_removed_and_unreadable_target_waits() {
 let (mut game,pos,source,target)=fixture();
 assert_eq!(tconstruct_source_faucet_tank::activate(&mut game,pos,source,target,Faucet::new()),Some(true));
 let reserved=Faucet::from_nbt(&tconstruct_source_melter_store::read(&game.world,pos,"tconstruct:faucet").unwrap()).unwrap();
 game.world.block_entities.remove(&target); let snapshot=game.world.tags.clone();
 assert!(tconstruct_source_faucet_tank::tick(&mut game,pos,source,target,reserved.clone())); assert_eq!(game.world.tags,snapshot);
 game.world.block_entities.insert(target,containers::BlockEntity::Chest(vec![]));
 game.world.blocks.remove(&source); game.world.block_entities.remove(&source);
 assert!(tconstruct_source_faucet_tank::tick(&mut game,pos,source,target,reserved));
 let pending=Faucet::from_nbt(&tconstruct_source_melter_store::read(&game.world,pos,"tconstruct:faucet").unwrap()).unwrap();
 let out=Tank::from_nbt(&tconstruct_source_melter_store::read(&game.world,target,"tconstruct:ingot_tank").unwrap(),4320).unwrap();
 assert_eq!((pending.pending.amount,out.amount),(70,20));
}
#[test] fn melter_source_preserves_metadata_and_unsupported_routes_fall_through() {
 let (mut game,pos,source,target)=fixture(); game.world.blocks.insert(source,1);
 game.world.tags.remove(&(source,"tconstruct:fuel_tank".into()));
 let mut melter=tconstruct_source_melter_state::Melter::new(); melter.fuel=33; assert!(melter.tank.insert("x:copper",90));
 let mut raw=melter.to_nbt();
 if let Tag::Compound(fields)=&mut raw { fields.insert("extra_test".into(),Tag::Int(7)); }
 game.world.tags.insert((source,"tconstruct:melter".into()),raw);
 assert_eq!(tconstruct_source_faucet_tank::activate(&mut game,pos,source,target,Faucet::new()),Some(true));
 let after=tconstruct_source_melter_state::Melter::from_nbt(&tconstruct_source_melter_store::read(&game.world,source,"tconstruct:melter").unwrap()).unwrap();
 assert_eq!((after.tank.amount,after.fuel),(0,33));
 assert_eq!(tconstruct_source_melter_store::read(&game.world,source,"tconstruct:melter").unwrap().get("extra_test"),Some(&Tag::Int(7)));
 game.world.blocks.remove(&target);
 assert_eq!(tconstruct_source_faucet_tank::activate(&mut game,pos,source,target,Faucet::new()),None);
}
#[test] fn preserves_unknown_source_target_and_faucet_fields() {
 let (mut game,pos,source,target)=fixture();
 for (p,key) in [(source,"tconstruct:fuel_tank"),(target,"tconstruct:ingot_tank")] {
  if let Some(Tag::Compound(fields))=game.world.tags.get_mut(&(p,key.into())) {fields.insert("extension".into(),Tag::Int(71));}
 }
 if let Some(Tag::Compound(fields))=game.world.tags.get_mut(&(pos,"tconstruct:faucet".into())) {
  fields.insert("extension".into(),Tag::Int(72));
  if let Some(Tag::Compound(pending))=fields.get_mut(&"pending".into()) {pending.insert("extension".into(),Tag::Int(73));}
 }
 assert_eq!(tconstruct_source_faucet_tank::activate(&mut game,pos,source,target,Faucet::new()),Some(true));
 let f=Faucet::from_nbt(&tconstruct_source_melter_store::read(&game.world,pos,"tconstruct:faucet").unwrap()).unwrap();
 assert!(tconstruct_source_faucet_tank::tick(&mut game,pos,source,target,f));
 for (p,key) in [(source,"tconstruct:fuel_tank"),(target,"tconstruct:ingot_tank")] {
  assert_eq!(game.world.tags.get(&(p,key.into())).unwrap().get("extension"),Some(&Tag::Int(71)));
 }
 let raw=game.world.tags.get(&(pos,"tconstruct:faucet".into())).unwrap();
 assert_eq!(raw.get("extension"),Some(&Tag::Int(72)));
 assert_eq!(raw.get("pending").unwrap().get("extension"),Some(&Tag::Int(73)));
}
#[test] fn preserves_unknown_melter_inner_tank_fields() {
 let (mut game,pos,source,target)=fixture(); game.world.blocks.insert(source,1);
 game.world.tags.remove(&(source,"tconstruct:fuel_tank".into()));
 let mut melter=tconstruct_source_melter_state::Melter::new(); assert!(melter.tank.insert("x:copper",90));
 let mut raw=melter.to_nbt();
 if let Tag::Compound(fields)=&mut raw {if let Some(Tag::Compound(tank))=fields.get_mut(&"tank".into()) { tank.insert("extension".into(),Tag::Int(74)); }}
 game.world.tags.insert((source,"tconstruct:melter".into()),raw);
 assert_eq!(tconstruct_source_faucet_tank::activate(&mut game,pos,source,target,Faucet::new()),Some(true));
 assert_eq!(game.world.tags.get(&(source,"tconstruct:melter".into())).unwrap().get("tank").unwrap().get("extension"),Some(&Tag::Int(74)));
}
#[test] fn each_new_packet_pours_immediately() {
 let (mut game,pos,source,target)=fixture();
 assert_eq!(tconstruct_source_faucet_tank::activate(&mut game,pos,source,target,Faucet::new()),Some(true));
 for _ in 0..8 {
  let faucet=Faucet::from_nbt(&tconstruct_source_melter_store::read(&game.world,pos,"tconstruct:faucet").unwrap()).unwrap();
  assert!(tconstruct_source_faucet_tank::tick(&mut game,pos,source,target,faucet));
 }
 let read=|game:&Game<'_>| Faucet::from_nbt(&tconstruct_source_melter_store::read(&game.world,pos,"tconstruct:faucet").unwrap()).unwrap();
 assert_eq!(read(&game).pending.amount,0);
 let faucet=read(&game); assert!(tconstruct_source_faucet_tank::tick(&mut game,pos,source,target,faucet));
 let out=Tank::from_nbt(&tconstruct_source_melter_store::read(&game.world,target,"tconstruct:ingot_tank").unwrap(),4320).unwrap();
 assert_eq!((read(&game).pending.amount,out.amount),(80,100));
}
#[test] fn stop_existing_faucet_preserves_faucet_and_pending_extensions() {
 let (mut game,pos,source,target)=fixture();
 if let Some(Tag::Compound(fields))=game.world.tags.get_mut(&(pos,"tconstruct:faucet".into())) {
  fields.insert("extension".into(),Tag::Int(21));
  if let Some(Tag::Compound(pending))=fields.get_mut(&"pending".into()) {pending.insert("extension".into(),Tag::Int(22));}
 }
 assert_eq!(tconstruct_source_faucet_tank::activate(&mut game,pos,source,target,Faucet::new()),Some(true));
 let f=Faucet::from_nbt(&tconstruct_source_melter_store::read(&game.world,pos,"tconstruct:faucet").unwrap()).unwrap();
 assert!(tconstruct_source_faucet_tank::stop(&mut game,pos,f));
 let raw=game.world.tags.get(&(pos,"tconstruct:faucet".into())).unwrap();
 assert_eq!(raw.get("extension"),Some(&Tag::Int(21)));
 assert_eq!(raw.get("pending").unwrap().get("extension"),Some(&Tag::Int(22)));
 assert!(Faucet::from_nbt(raw).unwrap().stop_after_packet);
}
#[test] fn faucet_codec_retains_extensions_through_legacy_target_removal() {
 let mut faucet=Faucet::new(); let mut source=Tank::new(90); assert!(source.insert("x:copper",90)); let target=Tank::new(90);
 assert!(faucet.activate(&mut source,&target));
 let mut raw=faucet.to_nbt(); if let Tag::Compound(fields)=&mut raw {
  fields.insert("extension".into(),Tag::Int(31));
  if let Some(Tag::Compound(pending))=fields.get_mut(&"pending".into()) {pending.insert("extension".into(),Tag::Int(32));}
 }
 let mut restored=Faucet::from_nbt(&raw).unwrap(); restored.tick(&mut source,None);
 let saved=restored.to_nbt(); assert_eq!(saved.get("extension"),Some(&Tag::Int(31)));
 assert_eq!(saved.get("pending").unwrap().get("extension"),Some(&Tag::Int(32)));
 assert_eq!(Faucet::from_nbt(&saved).unwrap().pending.amount,0);
}
#[test] fn melter_tank_only_patch_preserves_opaque_machine_and_slots() {
 use tconstruct_source_melter_state::Melter;
 let mut state=Melter::new(); state.slots[0]=Some(horizon_items::items::ItemStack::new("minecraft:copper_ore", 1)); assert!(state.tank.insert("x:copper",180)); let mut raw=state.to_nbt();
 if let Tag::Compound(fields)=&mut raw {
  fields.insert("extension".into(),Tag::Int(123));
  if let Some(Tag::List(slots))=fields.get_mut(&"slots".into()) { if let Tag::Compound(slot)=&mut slots[0] {slot.insert("opaque".into(),Tag::Int(7));} }
  if let Some(Tag::Compound(tank))=fields.get_mut(&"tank".into()) { tank.insert("extension".into(),Tag::Int(456)); }
 }
 assert!(Melter::from_nbt(&raw).is_some()); let original_slots=raw.get("slots").cloned(); assert_eq!(state.tank.take(90).as_deref(),Some("x:copper"));
 let result=tconstruct_source_faucet_tank::patch_melter_tank(&raw,&state.tank).unwrap();
 assert_eq!(result.get("extension"),Some(&Tag::Int(123)));assert_eq!(result.get("slots"),original_slots.as_ref());assert_eq!(result.get("tank").unwrap().get("extension"),Some(&Tag::Int(456)));assert_eq!(result.get("tank").unwrap().get("amount"),Some(&Tag::Int(90)));
 let mut bad=raw.clone();if let Tag::Compound(fields)=&mut bad {fields.insert("tank".into(),Tag::Int(1));}assert!(tconstruct_source_faucet_tank::patch_melter_tank(&bad,&state.tank).is_none());
}

#[test] fn melter_patch_drains_tagged_fluid_to_zero_without_erasing_extensions() {
 use tconstruct_source_melter_state::Melter;
 let mut state=Melter::new();let fluid_tag=Tag::compound([("Potion",Tag::String("minecraft:water".into()))]);assert!(state.tank.insert_tagged("x:potion",Some(&fluid_tag),90));let mut raw=state.to_nbt();if let Tag::Compound(fields)=&mut raw {fields.insert("heat_codes".into(),Tag::IntArray(vec![7,8,9]));if let Some(Tag::Compound(tank))=fields.get_mut(&"tank".into()) {tank.insert("extension".into(),Tag::Int(42));}}
 assert_eq!(state.tank.take_tagged(Some(&fluid_tag),90).as_deref(),Some("x:potion"));let out=tconstruct_source_faucet_tank::patch_melter_tank(&raw,&state.tank).unwrap();assert_eq!(out.get("heat_codes"),raw.get("heat_codes"));let tank=out.get("tank").unwrap();assert_eq!(tank.get("amount"),Some(&Tag::Int(0)));assert_eq!(tank.get("extension"),Some(&Tag::Int(42)));assert!(tank.get("fluid").is_none());assert!(tank.get("fluid_tag").is_none());
}
#[test] fn unreadable_casting_target_waits_and_removed_target_keeps_existing_behavior() {
 use tconstruct_source_melter_state::is_casting_block;
 for name in ["tconstruct:seared_table","tconstruct:scorched_table","tconstruct:seared_basin"] {
  let mut source=Tank::new(90); assert!(source.insert("x:copper",90)); let output=Tank::new(90); let mut faucet=Faucet::new();assert!(faucet.activate(&mut source,&output));
  let before=faucet.to_nbt();if !is_casting_block(name) {faucet.tick(&mut source,None);}assert_eq!(faucet.to_nbt(),before);
  let mut restored=Faucet::from_nbt(&faucet.to_nbt()).unwrap();let mut repaired=Tank::new(90);restored.tick(&mut source,Some(&mut repaired));assert_eq!((restored.pending.amount,repaired.amount),(80,10));
  if !is_casting_block("minecraft:air") {faucet.tick(&mut source,None);}assert!(!faucet.active);assert_eq!(faucet.pending.amount,0);
 }
}

#[test] fn lossy_source_id_does_not_reserve_or_change_any_state() {
 let (mut game,pos,source,target)=fixture();
 let raw=Tag::compound([("fluid",Tag::String(horizon_world::nbt::JavaString(vec![0xd800]))),("amount",Tag::Int(180)),("future",Tag::Int(7))]);
 game.world.tags.insert((source,"tconstruct:fuel_tank".into()),raw);
 let before=game.world.tags.clone();
 assert_eq!(tconstruct_source_faucet_tank::activate(&mut game,pos,source,target,Faucet::new()),Some(false));
 assert_eq!(game.world.tags,before);
}
#[test] fn lossy_reserved_packet_is_not_rewritten_when_stopping() {
 let (mut game,pos,_source,_target)=fixture();
 let pending=Tag::compound([("fluid",Tag::String(horizon_world::nbt::JavaString(vec![0xd800]))),("amount",Tag::Int(10))]);
 let mut raw=Faucet::new().to_nbt(); if let Tag::Compound(fields)=&mut raw {fields.insert("active".into(),Tag::Byte(1));fields.insert("pending".into(),pending);}
 game.world.tags.insert((pos,"tconstruct:faucet".into()),raw.clone());
 assert!(Faucet::from_nbt(&raw).is_none());
 // Even a stale previously decoded state cannot rewrite the invalid save.
 assert!(!tconstruct_source_faucet_tank::stop(&mut game,pos,Faucet::new()));
 assert_eq!(game.world.tags.get(&(pos,"tconstruct:faucet".into())),Some(&raw));
}
#[test] fn lossy_target_id_does_not_drain_the_valid_source() {
 let (mut game,pos,source,target)=fixture();
 let mut input=Tank::new(4000); assert!(input.insert("�",180));
 game.world.tags.insert((source,"tconstruct:fuel_tank".into()),input.to_nbt());
 let raw=Tag::compound([("fluid",Tag::String(horizon_world::nbt::JavaString(vec![0xd800]))),("amount",Tag::Int(10)),("future",Tag::Int(7))]);
 game.world.tags.insert((target,"tconstruct:ingot_tank".into()),raw);
 let before=game.world.tags.clone();
 assert_eq!(tconstruct_source_faucet_tank::activate(&mut game,pos,source,target,Faucet::new()),Some(false));
 assert_eq!(game.world.tags,before);
}
#[test] fn invalid_source_keeps_already_reserved_packet_waiting() {
 let (mut game,pos,source,target)=fixture();
 assert_eq!(tconstruct_source_faucet_tank::activate(&mut game,pos,source,target,Faucet::new()),Some(true));
 let faucet=Faucet::from_nbt(game.world.tags.get(&(pos,"tconstruct:faucet".into())).unwrap()).unwrap();
 assert!(faucet.pending.amount>0);
 let invalid=Tag::compound([("fluid",Tag::String(horizon_world::nbt::JavaString(vec![0xd800]))),("amount",Tag::Int(10))]);
 game.world.tags.insert((source,"tconstruct:fuel_tank".into()),invalid);
 let before=game.world.tags.clone();
 assert!(tconstruct_source_faucet_tank::tick(&mut game,pos,source,target,faucet));
 assert_eq!(game.world.tags,before);
}
