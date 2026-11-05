// Production Cannon and Casting Tank routes, with bounded host callbacks.
#[path="__MIXINS__/source-smeltery-fuel.rs"] mod tconstruct_source_smeltery_fuel;
#[path="__MIXINS__/source-melter-state.rs"] mod tconstruct_source_melter_state;
#[path="__MIXINS__/source-smeltery-tank.rs"] mod tconstruct_source_smeltery_tank;
#[path="__MIXINS__/source-casting-tank.rs"] mod tconstruct_source_casting_tank;
#[path="__MIXINS__/source-fluid-cannon.rs"] mod tconstruct_source_fluid_cannon;
#[path="__MIXINS__/source-fluid-container.rs"] mod tconstruct_source_fluid_container;
#[path="__MIXINS__/source-fluid-display.rs"] mod tconstruct_source_fluid_display;
#[path="__PLUTO__"] mod pluto_stack_fluid;
use std::collections::{HashMap,HashSet};
use horizon_world::nbt::Tag;use horizon_items::items::ItemStack;
type Pos=(i32,i32,i32);type Tags=HashMap<String,HashSet<String>>;
#[derive(Clone)]enum BlockEntity{Chest(Vec<Option<ItemStack>>)}
struct Items;impl Items{fn get(&self,_:&str)->Option<()>{Some(())}}
struct Res;impl Res{fn items(&self)->Items{Items}}
struct Chunks{loaded:bool,fluids:Option<Result<(HashSet<String>,()),()>>,tags:Option<Result<Tags,()>>}
impl Chunks{
 fn contains_key(&self,_:&(i32,i32))->bool{self.loaded}
 fn __horizon_mixins_tconstruct_tank_fluid_context(&self)->&Option<Result<(HashSet<String>,()),()>>{&self.fluids}
 fn __horizon_mixins_tconstruct_resolved_item_tags(&self)->&Option<Result<Tags,()>>{&self.tags}
}
struct World{raw:Tag,chunks:Chunks,block_entities:HashMap<Pos,BlockEntity>,block:usize,fail:bool,writes:usize}
impl World{fn block(&self,_:i32,_:i32,_:i32)->usize{self.block}}
struct Game<'a>{world:World,res:Res,held:Option<ItemStack>,gifts:Vec<ItemStack>,creative:bool,lan:Option<()>,lan_acting_uuid:Option<()>,fault:u8,fault_after:usize,preparations:usize,signals:usize,updates:usize,_life:std::marker::PhantomData<&'a()>}
impl Game<'_>{
 fn held(&self)->Option<&ItemStack>{self.held.as_ref()}fn held_mut(&mut self)->&mut Option<ItemStack>{&mut self.held}
 fn give_or_drop(&mut self,item:ItemStack){self.gifts.push(item)}fn container_changed(&mut self,_:Pos){self.updates+=1;}
 fn ensure_datapacks(&mut self){
  self.preparations+=1;if self.preparations!=self.fault_after{return;}
  match self.fault{
   1=>self.held.as_mut().unwrap().count+=1,
   2=>{let BlockEntity::Chest(slots)=self.world.block_entities.get_mut(&(0,0,0)).unwrap();slots[0]=Some(ItemStack::new("minecraft:diamond",1));},
   3=>if let Tag::Compound(fields)=&mut self.world.raw{fields.insert("future".into(),Tag::Int(999));},
   4=>self.world.block=99,
   5=>self.world.chunks.loaded=false,
   6=>self.creative=!self.creative,
   7=>{let BlockEntity::Chest(slots)=self.world.block_entities.get_mut(&(0,0,0)).unwrap();let item=slots[0].get_or_insert_with(||ItemStack::new("minecraft:stone",1));item.data_mut().wire=horizon_items::item_data::Wire::new(item.clone(),std::sync::Arc::<[u8]>::from([1,2,3]));item.data_mut().set_other("minecraft:custom_name",Some(Tag::String("Changed".into())));},
   8=>{let item=self.held.as_mut().unwrap();item.data_mut().wire=horizon_items::item_data::Wire::new(item.clone(),std::sync::Arc::<[u8]>::from([1,2,3]));item.data_mut().set_other("minecraft:custom_name",Some(Tag::String("Changed".into())));},
   9=>self.lan=Some(()),
   10=>self.lan_acting_uuid=Some(()),
   11=>{let BlockEntity::Chest(slots)=self.world.block_entities.get_mut(&(0,0,0)).unwrap();slots[1]=Some(ItemStack::new("minecraft:diamond",1));},
   12=>if let Tag::Compound(fields)=&mut self.world.raw{if let Some(Tag::Compound(tank))=fields.get_mut(&"tank".into()){tank.insert("amount".into(),Tag::Int(50));tank.insert("fluid".into(),Tag::String("minecraft:water".into()));}},
   13=>{self.world.block_entities.remove(&(0,0,0));},
   _=>{},
  }
 }
}
mod store{use super::*;pub(super)fn read(world:&World,_:Pos,_:&str)->Option<Tag>{world.chunks.loaded.then(||world.raw.clone())}pub(super)fn write(world:&mut World,_:Pos,_:&str,raw:Tag)->bool{if world.fail{return false}world.raw=raw;world.writes+=1;true}}
mod runtime{
 use super::*;
 fn ensure(_: &mut Game<'_>,_:Pos,_:&str)->bool{true}
 fn notify_casting_comparator(game:&mut Game<'_>,_:Pos,before:i32,after:i32){if before!=after{game.signals+=1;}}
 // __CANNON_AT__
 // __CANNON_SAVE__
 // __CASTING_TANK_SAVE__
 // __CASTING_TANK_INTERACTION__
 // __CANNON_HELD__
}
fn fixture(draining:bool,count:u32)->Game<'static>{
 use tconstruct_source_fluid_cannon::Cannon;
 let item=ItemStack::new("minecraft:stone",16);let mut state=Cannon::new();state.item=Some(item.clone());
 if draining{assert!(state.tank.insert("minecraft:water",1000));}
 let mut raw=state.to_nbt();if let Tag::Compound(fields)=&mut raw{
  fields.insert("future".into(),Tag::Int(7));
  if let Some(Tag::Compound(tank))=fields.get_mut(&"tank".into()){tank.insert("future_tank".into(),Tag::Int(8));}
  if let Some(Tag::Compound(item))=fields.get_mut(&"item".into()){item.insert("future_item".into(),Tag::Int(9));}
 }
 Game{world:World{raw,chunks:Chunks{loaded:true,fluids:Some(Ok((HashSet::from(["minecraft:water".into()]),()))),tags:Some(Ok(Tags::new()))},block_entities:HashMap::from([((0,0,0),BlockEntity::Chest(vec![Some(item)]))]),block:1,fail:false,writes:0},res:Res,held:Some(ItemStack::new(if draining{"minecraft:bucket"}else{"minecraft:water_bucket"},count)),gifts:vec![],creative:false,lan:None,lan_acting_uuid:None,fault:0,fault_after:1,preparations:0,signals:0,updates:0,_life:std::marker::PhantomData}
}
#[test]fn cannon_held_fill_drain_preserve_raw_and_live_items(){
 for draining in [false,true]{for count in [1,2]{for creative in [false,true]{
  let mut game=fixture(draining,count);game.creative=creative;let raw=game.world.raw.clone();let held=game.held.clone();
  let BlockEntity::Chest(before)=game.world.block_entities[&(0,0,0)].clone();
  assert!(runtime::interact_cannon(&mut game,(0,0,0),true));
  let state=tconstruct_source_fluid_cannon::Cannon::from_nbt(&game.world.raw).unwrap();assert_eq!(state.tank.amount,if draining{0}else{1000});
  assert_eq!(game.world.raw.get("future"),raw.get("future"));assert_eq!(game.world.raw.get("item"),raw.get("item"));
  assert_eq!(game.world.raw.get("tank").unwrap().get("future_tank"),Some(&Tag::Int(8)));
  let BlockEntity::Chest(after)=&game.world.block_entities[&(0,0,0)];assert_eq!(after,&before);
  assert_eq!((game.world.writes,game.signals,game.updates),(1,1,1));
  if creative{assert_eq!(game.held,held);assert_eq!(game.gifts.len(),usize::from(draining));}
  else if count==2{assert_eq!(game.held.as_ref().unwrap().count,1);assert_eq!(game.gifts.len(),1);}
  else{assert_eq!(&*game.held.as_ref().unwrap().item,if draining{"minecraft:water_bucket"}else{"minecraft:bucket"});assert!(game.gifts.is_empty());}
 }}}
}
#[test]fn cannon_held_snapshot_changes_never_commit_or_notify(){
 for fault in 1..=8{let mut game=fixture(false,1);game.fault=fault;
  assert!(runtime::interact_cannon(&mut game,(0,0,0),true));
  assert_eq!((game.world.writes,game.signals,game.updates),(0,0,0));assert!(game.gifts.is_empty());
  assert_eq!(tconstruct_source_fluid_cannon::Cannon::from_nbt(&game.world.raw).unwrap().tank.amount,0);
  assert_eq!(&*game.held.as_ref().unwrap().item,"minecraft:water_bucket");
 }
}
#[test]fn cannon_held_unsafe_metadata_and_failed_save_preserve_inventory(){
 for mode in 0..6 {let mut game=fixture(false,1);
  if mode<4{if let Tag::Compound(fields)=&mut game.world.raw{match mode{
   0=>{fields.insert("Tank".into(),Tag::Int(1));},
   1=>{if let Some(Tag::Compound(tank))=fields.get_mut(&"tank".into()){tank.insert("Amount".into(),Tag::Int(1));}},
   2=>{if let Some(Tag::Compound(tank))=fields.get_mut(&"tank".into()){tank.insert("amount".into(),Tag::Int(10));tank.insert("fluid".into(),Tag::String(horizon_world::nbt::JavaString(vec![0xd800])));}},
   3=>{fields.insert("item".into(),horizon_items::item_nbt::item_to_nbt(&ItemStack::new("minecraft:diamond",1)));},
   _=>unreachable!(),
  }}}else if mode==4{let item=game.held.as_mut().unwrap();item.data_mut().wire=horizon_items::item_data::Wire::new(item.clone(),std::sync::Arc::<[u8]>::from([1,2,3]));}else{game.world.fail=true;}
  let before=(game.world.raw.clone(),game.held.clone());let BlockEntity::Chest(slots)=game.world.block_entities[&(0,0,0)].clone();
  runtime::interact_cannon(&mut game,(0,0,0),true);
  assert_eq!((game.world.raw,game.held),before);let BlockEntity::Chest(after)=&game.world.block_entities[&(0,0,0)];assert_eq!(after,&slots);
  assert_eq!((game.world.writes,game.signals,game.updates),(0,0,0));assert!(game.gifts.is_empty());
 }
}

fn canonical_saved_item(game:&mut Game<'_>){
 if let Tag::Compound(fields)=&mut game.world.raw{if let Some(Tag::Compound(item))=fields.get_mut(&"item".into()){item.remove(&"future_item".into());}}
}
#[test]fn cannon_item_saves_preserve_extensions_before_publishing_slots(){
 let mut game=fixture(false,1);canonical_saved_item(&mut game);let initial=game.world.raw.clone();
 for next in [Some(ItemStack::new("minecraft:stone",15)),None,Some(ItemStack::new("minecraft:diamond",32))]{
  let mut state=runtime::cannon_at(&game,(0,0,0)).unwrap();state.item=next.clone();
  assert!(runtime::save_cannon(&mut game,(0,0,0),&state));
  assert_eq!(game.world.raw.get("future"),initial.get("future"));assert_eq!(game.world.raw.get("tank"),initial.get("tank"));
  let restored=tconstruct_source_fluid_cannon::Cannon::from_nbt(&game.world.raw).unwrap();assert_eq!(restored.item,next);
  let BlockEntity::Chest(live)=&game.world.block_entities[&(0,0,0)];assert_eq!(live[0],next);
 }
 assert_eq!((game.world.writes,game.signals,game.updates),(3,0,3));
}
#[test]fn cannon_item_saves_refuse_opaque_changes_and_stale_live_state(){
 for mode in 0..5{let mut game=fixture(false,1);let mut state=runtime::cannon_at(&game,(0,0,0)).unwrap();
  match mode{
   0=>state.item=None,
   1=>state.item.as_mut().unwrap().count-=1,
   2=>state.item=Some(ItemStack::new("minecraft:diamond",1)),
   3=>{canonical_saved_item(&mut game);if let Tag::Compound(fields)=&mut game.world.raw{fields.insert("Items".into(),Tag::List(vec![]));}state.item=None;},
   4=>{canonical_saved_item(&mut game);let BlockEntity::Chest(slots)=game.world.block_entities.get_mut(&(0,0,0)).unwrap();slots[0]=Some(ItemStack::new("minecraft:diamond",1));},
   _=>unreachable!(),
  }
  let raw=game.world.raw.clone();let BlockEntity::Chest(before)=game.world.block_entities[&(0,0,0)].clone();
  assert!(!runtime::save_cannon(&mut game,(0,0,0),&state));assert_eq!(game.world.raw,raw);
  let BlockEntity::Chest(after)=&game.world.block_entities[&(0,0,0)];assert_eq!(after,&before);
  assert_eq!((game.world.writes,game.signals,game.updates),(0,0,0));
 }
 // The same opaque item can remain byte-for-byte in an unchanged slot.
 let mut game=fixture(false,1);let raw=game.world.raw.clone();let state=runtime::cannon_at(&game,(0,0,0)).unwrap();
 assert!(runtime::save_cannon(&mut game,(0,0,0),&state));assert_eq!(game.world.raw,raw);
}
#[test]fn cannon_item_saves_refuse_retained_wire_and_failed_persistence(){
 for mode in 0..3{let mut game=fixture(false,1);canonical_saved_item(&mut game);let mut state=runtime::cannon_at(&game,(0,0,0)).unwrap();
  state.item=Some(ItemStack::new("minecraft:diamond",1));
  if mode<2{let item=state.item.as_mut().unwrap();item.data_mut().wire=horizon_items::item_data::Wire::new(item.clone(),std::sync::Arc::<[u8]>::from([1,2,3]));if mode==1{item.data_mut().set_other("minecraft:custom_name",Some(Tag::String("Changed".into())));}assert!(!state.lossless());}else{game.world.fail=true;}
  let raw=game.world.raw.clone();let BlockEntity::Chest(before)=game.world.block_entities[&(0,0,0)].clone();
  assert!(!runtime::save_cannon(&mut game,(0,0,0),&state));assert_eq!(game.world.raw,raw);
  let BlockEntity::Chest(after)=&game.world.block_entities[&(0,0,0)];assert_eq!(after,&before);
  assert_eq!((game.world.writes,game.signals,game.updates),(0,0,0));
 }
}

fn casting_fixture(input:Option<ItemStack>,output:Option<ItemStack>,opaque:bool)->Game<'static>{
 let mut game=fixture(false,1);let mut state=tconstruct_source_casting_tank::CastingTank::new();state.slots=[input,output];
 let mut raw=state.to_nbt();if let Tag::Compound(fields)=&mut raw{
  fields.insert("future".into(),Tag::Int(7));if let Some(Tag::Compound(tank))=fields.get_mut(&"tank".into()){tank.insert("future_tank".into(),Tag::Int(8));}
  if opaque{if let Some(Tag::List(slots))=fields.get_mut(&"slots".into()){for (index,slot) in slots.iter_mut().enumerate(){if state.slots[index].is_some(){if let Tag::Compound(fields)=slot{fields.insert("future_item".into(),Tag::Int(9));}}}}}
 }
 game.world.raw=raw;game.world.block_entities.insert((0,0,0),BlockEntity::Chest(state.slots.to_vec()));game
}
#[test]fn casting_tank_save_preserves_extensions_and_processed_containers(){
 let mut game=casting_fixture(Some(ItemStack::new("minecraft:water_bucket",1)),None,false);let raw=game.world.raw.clone();
 let mut state=runtime::casting_tank_at(&game,(0,0,0)).unwrap();assert!(state.process(&|_|true,&|_|true,None,false));
 assert!(runtime::save_casting_tank(&mut game,(0,0,0),&state));
 assert_eq!(state.tank.amount,1000);assert_eq!(game.world.raw.get("future"),raw.get("future"));
 assert_eq!(game.world.raw.get("tank").unwrap().get("future_tank"),Some(&Tag::Int(8)));
 let BlockEntity::Chest(slots)=&game.world.block_entities[&(0,0,0)];assert_eq!(slots,&state.slots.to_vec());
 assert_eq!((game.world.writes,game.signals,game.updates),(1,1,1));
 state.slots[1]=None;state.redstone=true;assert!(runtime::save_casting_tank(&mut game,(0,0,0),&state));
 let restored=tconstruct_source_casting_tank::CastingTank::from_nbt(&game.world.raw).unwrap();assert!(restored.redstone);assert!(restored.slots.iter().all(Option::is_none));
 assert_eq!((game.world.writes,game.signals,game.updates),(2,1,2));
}
#[test]fn casting_tank_save_refuses_opaque_slot_changes_and_stale_state(){
 for mode in 0..6{let mut game=casting_fixture(Some(ItemStack::new("minecraft:water_bucket",1)),Some(ItemStack::new("minecraft:stone",2)),true);
  let mut state=runtime::casting_tank_at(&game,(0,0,0)).unwrap();
  match mode{
   0=>state.slots[0]=None,
   1=>state.slots[1].as_mut().unwrap().count-=1,
   2=>state.slots[1]=Some(ItemStack::new("minecraft:diamond",1)),
   3=>{let BlockEntity::Chest(slots)=game.world.block_entities.get_mut(&(0,0,0)).unwrap();slots[0]=None;},
   4=>if let Tag::Compound(fields)=&mut game.world.raw{fields.insert("Items".into(),Tag::List(vec![]));},
   5=>if let Tag::Compound(fields)=&mut game.world.raw{if let Some(Tag::Compound(tank))=fields.get_mut(&"tank".into()){tank.insert("Amount".into(),Tag::Int(1));}},
   _=>unreachable!(),
  }
  let raw=game.world.raw.clone();let BlockEntity::Chest(before)=game.world.block_entities[&(0,0,0)].clone();
  assert!(!runtime::save_casting_tank(&mut game,(0,0,0),&state));assert_eq!(game.world.raw,raw);
  let BlockEntity::Chest(after)=&game.world.block_entities[&(0,0,0)];assert_eq!(after,&before);
  assert_eq!((game.world.writes,game.signals,game.updates),(0,0,0));
 }
 let mut game=casting_fixture(None,Some(ItemStack::new("minecraft:stone",1)),true);let raw=game.world.raw.clone();let mut state=runtime::casting_tank_at(&game,(0,0,0)).unwrap();
 assert!(state.tank.insert("minecraft:water",50));assert!(runtime::save_casting_tank(&mut game,(0,0,0),&state));assert_eq!(game.world.raw.get("slots"),raw.get("slots"));
}
#[test]fn casting_tank_process_and_save_refuse_wire_and_failed_commit(){
 for mode in 0..3{let mut game=casting_fixture(Some(ItemStack::new("minecraft:water_bucket",1)),None,false);let mut state=runtime::casting_tank_at(&game,(0,0,0)).unwrap();
  if mode<2{let item=state.slots[0].as_mut().unwrap();item.data_mut().wire=horizon_items::item_data::Wire::new(item.clone(),std::sync::Arc::<[u8]>::from([1,2,3]));if mode==1{item.data_mut().set_other("minecraft:custom_name",Some(Tag::String("Changed".into())));}let before=state.to_nbt();assert!(!state.process(&|_|true,&|_|true,None,false));assert_eq!(state.to_nbt(),before);assert!(!state.lossless());}else{assert!(state.process(&|_|true,&|_|true,None,false));game.world.fail=true;}
  let raw=game.world.raw.clone();let BlockEntity::Chest(before)=game.world.block_entities[&(0,0,0)].clone();
  assert!(!runtime::save_casting_tank(&mut game,(0,0,0),&state));assert_eq!(game.world.raw,raw);
  let BlockEntity::Chest(after)=&game.world.block_entities[&(0,0,0)];assert_eq!(after,&before);assert_eq!((game.world.writes,game.signals,game.updates),(0,0,0));
 }
}

#[test]fn casting_tank_held_transfers_preserve_metadata_and_counts(){
 for draining in [false,true]{for count in [1,2]{for creative in [false,true]{
  let mut game=casting_fixture(Some(ItemStack::new("minecraft:stone",1)),Some(ItemStack::new("minecraft:stone",2)),true);
  if draining{let mut state=runtime::casting_tank_at(&game,(0,0,0)).unwrap();assert!(state.tank.insert("minecraft:water",1000));game.world.raw=state.to_nbt_preserving(&game.world.raw).unwrap();}
  game.held=Some(ItemStack::new(if draining{"minecraft:bucket"}else{"minecraft:water_bucket"},count));game.creative=creative;
  let raw=game.world.raw.clone();let held=game.held.clone();let BlockEntity::Chest(before)=game.world.block_entities[&(0,0,0)].clone();
  assert!(runtime::interact_casting_tank(&mut game,(0,0,0),true));
  let state=tconstruct_source_casting_tank::CastingTank::from_nbt(&game.world.raw).unwrap();assert_eq!(state.tank.amount,if draining{0}else{1000});
  assert_eq!(game.world.raw.get("slots"),raw.get("slots"));assert_eq!(game.world.raw.get("future"),raw.get("future"));
  assert_eq!(game.world.raw.get("tank").unwrap().get("future_tank"),Some(&Tag::Int(8)));
  let BlockEntity::Chest(after)=&game.world.block_entities[&(0,0,0)];assert_eq!(after,&before);
  assert_eq!((game.world.writes,game.signals,game.updates,game.preparations),(1,1,1,2));
  if creative{assert_eq!(game.held,held);assert_eq!(game.gifts.len(),usize::from(draining));}
  else if count==2{assert_eq!(game.held.as_ref().unwrap().count,1);assert_eq!(game.gifts.len(),1);}
  else{assert_eq!(&*game.held.as_ref().unwrap().item,if draining{"minecraft:water_bucket"}else{"minecraft:bucket"});assert!(game.gifts.is_empty());}
 }}}
 // The same commit includes automatic processing of the machine input.
 let mut game=casting_fixture(Some(ItemStack::new("minecraft:water_bucket",1)),None,false);
 assert!(runtime::interact_casting_tank(&mut game,(0,0,0),true));
 let state=runtime::casting_tank_at(&game,(0,0,0)).unwrap();assert_eq!(state.tank.amount,2000);assert!(state.slots[0].is_none());
 assert_eq!(&*state.slots[1].as_ref().unwrap().item,"minecraft:bucket");assert_eq!(&*game.held.as_ref().unwrap().item,"minecraft:bucket");
 assert_eq!(game.world.writes,1);
}
#[test]fn casting_tank_held_callbacks_never_overwrite_changed_state(){
 for fault_after in [1,2]{for fault in 1..=13{
  let mut game=casting_fixture(Some(ItemStack::new("minecraft:stone",1)),Some(ItemStack::new("minecraft:stone",2)),false);
  game.fault=fault;game.fault_after=fault_after;let raw=game.world.raw.clone();
  assert!(runtime::interact_casting_tank(&mut game,(0,0,0),true));
  assert_eq!((game.world.writes,game.signals,game.updates),(0,0,0));assert!(game.gifts.is_empty());
  if fault!=3 && fault!=12{assert_eq!(game.world.raw,raw);}
  assert_eq!(&*game.held.as_ref().unwrap().item,"minecraft:water_bucket");
  assert_eq!(game.held.as_ref().unwrap().count,if fault==1{2}else{1});
  if fault==2{let BlockEntity::Chest(slots)=&game.world.block_entities[&(0,0,0)];assert_eq!(&*slots[0].as_ref().unwrap().item,"minecraft:diamond");}
  if fault==11{let BlockEntity::Chest(slots)=&game.world.block_entities[&(0,0,0)];assert_eq!(&*slots[1].as_ref().unwrap().item,"minecraft:diamond");}
 }}
}
#[test]fn casting_tank_upper_insert_take_and_callback_guard(){
 for count in [1,2]{let mut game=casting_fixture(None,None,false);game.held.as_mut().unwrap().count=count;let raw=game.world.raw.clone();
  assert!(runtime::interact_casting_tank(&mut game,(0,0,0),false));
  let state=runtime::casting_tank_at(&game,(0,0,0)).unwrap();assert_eq!(state.tank.amount,1000);assert!(state.slots[0].is_none());
  assert_eq!(&*state.slots[1].as_ref().unwrap().item,"minecraft:bucket");assert_eq!(game.held.as_ref().map(|item|item.count),if count==1{None}else{Some(1)});
  assert_eq!(game.world.raw.get("future"),raw.get("future"));assert_eq!(game.world.raw.get("tank").unwrap().get("future_tank"),Some(&Tag::Int(8)));
  let held=game.held.clone();assert!(runtime::interact_casting_tank(&mut game,(0,0,0),false));assert_eq!(game.held,held);
  assert_eq!(&*game.gifts[0].item,"minecraft:bucket");assert!(runtime::casting_tank_at(&game,(0,0,0)).unwrap().slots.iter().all(Option::is_none));
  assert_eq!((game.world.writes,game.signals,game.updates),(2,1,2));
 }
 // An upper click prefers the output; a later click returns the input.
 let mut game=casting_fixture(Some(ItemStack::new("minecraft:bucket",1)),Some(ItemStack::new("minecraft:water_bucket",1)),false);game.held=None;
 assert!(runtime::interact_casting_tank(&mut game,(0,0,0),false));assert_eq!(&*game.gifts[0].item,"minecraft:water_bucket");
 assert!(runtime::interact_casting_tank(&mut game,(0,0,0),false));assert_eq!(&*game.gifts[1].item,"minecraft:bucket");
 for fault in 1..=13{let mut game=casting_fixture(None,None,false);game.fault=fault;let raw=game.world.raw.clone();
  assert!(runtime::interact_casting_tank(&mut game,(0,0,0),false));assert_eq!((game.world.writes,game.signals,game.updates),(0,0,0));assert!(game.gifts.is_empty());
  if fault!=3 && fault!=12{assert_eq!(game.world.raw,raw);}
  assert_eq!(&*game.held.as_ref().unwrap().item,"minecraft:water_bucket");assert_eq!(game.held.as_ref().unwrap().count,if fault==1{2}else{1});
 }
}
#[test]fn casting_tank_interaction_refuses_wire_and_failed_save(){
 for clicked_tank in [false,true]{for mode in 0..4{let mut game=casting_fixture(None,None,false);
  if mode<2{let item=game.held.as_mut().unwrap();item.data_mut().wire=horizon_items::item_data::Wire::new(item.clone(),std::sync::Arc::<[u8]>::from([1,2,3]));if mode==1{item.data_mut().set_other("minecraft:custom_name",Some(Tag::String("Changed".into())));}}
  else if mode==2{game.world.fail=true;}else if let Tag::Compound(fields)=&mut game.world.raw{fields.insert("Tank".into(),Tag::Int(1));}
  let raw=game.world.raw.clone();let held=game.held.clone();let BlockEntity::Chest(before)=game.world.block_entities[&(0,0,0)].clone();
  runtime::interact_casting_tank(&mut game,(0,0,0),clicked_tank);assert_eq!(game.world.raw,raw);assert_eq!(game.held,held);
  let BlockEntity::Chest(after)=&game.world.block_entities[&(0,0,0)];assert_eq!(after,&before);assert_eq!((game.world.writes,game.signals,game.updates),(0,0,0));assert!(game.gifts.is_empty());
 }}
}
