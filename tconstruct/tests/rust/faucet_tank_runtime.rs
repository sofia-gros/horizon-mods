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
