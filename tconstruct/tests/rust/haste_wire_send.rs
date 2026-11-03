use horizon_items::items::ItemStack;
use std::collections::{HashMap,HashSet,BTreeMap};
#[path = "../../mixins/tconstruct/source-dwarven.rs"] mod tconstruct_source_dwarven;
#[path = "../../mixins/tconstruct/source-mining-math.rs"] mod tconstruct_source_mining_math;
#[path = "../../mixins/tconstruct/source-haste-source.rs"] mod tconstruct_source_haste_source;
#[path = "../../../pluto-lib/mixins/pluto_tool_rules/rules.rs"] pub mod rules;
mod tconstruct_tool_api { pub use horizon_items::item_data::canon_tag; pub use crate::rules; }
mod tconstruct_source_material_traits {
 use super::*; pub struct Traits { pub levels:BTreeMap<String,i32> }
 pub fn levels(_: &lan::hub::World, stack:&ItemStack)->Option<Traits>{Some(Traits{levels:BTreeMap::from([("tconstruct:haste".into(),if &*stack.item=="test:a" {1} else {2})])})}
}
mod lan { pub mod hub {
 use super::super::*;use std::cell::RefCell; use horizon_protocol::packets::play::Clientbound as C;
 pub mod tconstruct_source_haste_wire { include!("../../mixins/tconstruct/source-haste-wire.rs"); }
 pub struct Guest { pub conn:u32,pub placed_at:u64,pub armor:[Option<ItemStack>;4],pub sent:RefCell<Vec<Vec<u8>>> }
 impl Guest {fn send(&self, packet:&C) { if let C::CustomPayload(m)=packet {self.sent.borrow_mut().push(m.data.clone());} }}
 pub struct Hub {pub guests:Vec<Guest>}
 impl Hub {fn here(&self,_:&Guest)->bool {true}}
 type Raw=(BTreeMap<String,Vec<Vec<u8>>>,BTreeMap<String,Vec<Vec<u8>>>);
 pub struct Chunks {pub generation:u64,pub cache:HashMap<(u32,u64),(u64,bool,Option<ItemStack>,(Vec<u8>,u8))>,pub tags:Option<Result<HashMap<String,HashSet<String>>,String>>,pub raw:Option<Result<Raw,String>>}
 impl Chunks {
  fn __horizon_mixins_tconstruct_source_recipe_generation(&self)->&u64{&self.generation}
  fn __horizon_mixins_tconstruct_haste_wire_sent(&self)->&HashMap<(u32,u64),(u64,bool,Option<ItemStack>,(Vec<u8>,u8))>{&self.cache}
  fn __horizon_mixins_tconstruct_haste_wire_sent_mut(&mut self)->&mut HashMap<(u32,u64),(u64,bool,Option<ItemStack>,(Vec<u8>,u8))>{&mut self.cache}
  fn __horizon_mixins_tconstruct_resolved_item_tags(&self)->&Option<Result<HashMap<String,HashSet<String>>,String>>{&self.tags}
  fn __horizon_mixins_tconstruct_modifier_resource_snapshot(&self)->&Option<Result<Raw,String>>{&self.raw}
  fn __horizon_mixins_tconstruct_material_snapshot(&self)->Option<()> {Some(())}
  fn __horizon_mixins_tconstruct_recipe_snapshot(&self)->Option<()> {Some(())}
 }
 pub struct World {pub chunks:Chunks}
 pub struct TickCtx<'a> {pub world:&'a mut World}
 include!("../../mixins/tconstruct/source-haste-wire-send.rs");
 #[test] fn separate_actors_equipment_reload_and_disconnect() {
  let mk=|conn,item| Guest{conn,placed_at:1,armor:[None,Some(ItemStack::new(item,1)),None,None],sent:RefCell::new(Vec::new())};
  let mut h=Hub{guests:vec![mk(1,"test:a"),mk(2,"test:b")]};
  let mut w=World{chunks:Chunks{generation:1,cache:HashMap::new(),tags:Some(Ok(HashMap::from([("tconstruct:modifiable/harvest".into(),HashSet::new())]))),raw:Some(Ok((BTreeMap::from([("tconstruct:tinkering/modifiers/haste".into(),vec![crate::tconstruct_source_haste_source::PINNED_HASTE.to_vec()])]),BTreeMap::new())))}};
  tconstruct_haste_wire_send(&mut h,&mut TickCtx{world:&mut w});
  assert_eq!(tconstruct_source_haste_wire::decode(&h.guests[0].sent.borrow()[0]).unwrap().1,1);
  assert_eq!(tconstruct_source_haste_wire::decode(&h.guests[1].sent.borrow()[0]).unwrap().1,2);
  h.guests[0].armor[1]=None;tconstruct_haste_wire_send(&mut h,&mut TickCtx{world:&mut w});
  assert!(tconstruct_source_haste_wire::decode(h.guests[0].sent.borrow().last().unwrap()).is_none());
  w.chunks.tags=None;tconstruct_haste_wire_send(&mut h,&mut TickCtx{world:&mut w});
  assert!(tconstruct_source_haste_wire::decode(h.guests[1].sent.borrow().last().unwrap()).is_none());
  // Restoring tags with no generation change makes the next snapshot known.
  w.chunks.tags=Some(Ok(HashMap::from([("tconstruct:modifiable/harvest".into(),HashSet::new())])));
  tconstruct_haste_wire_send(&mut h,&mut TickCtx{world:&mut w});
  assert_eq!(tconstruct_source_haste_wire::decode(h.guests[1].sent.borrow().last().unwrap()).unwrap().1,2);
  // An override invalidates a previously known result, also before any bump.
  w.chunks.raw.as_mut().unwrap().as_mut().unwrap().0.get_mut("tconstruct:tinkering/modifiers/haste").unwrap().push(b"{}".to_vec());
  tconstruct_haste_wire_send(&mut h,&mut TickCtx{world:&mut w});
  assert!(tconstruct_source_haste_wire::decode(h.guests[1].sent.borrow().last().unwrap()).is_none());
  h.guests.remove(0);tconstruct_haste_wire_send(&mut h,&mut TickCtx{world:&mut w});assert!(!w.chunks.cache.contains_key(&(1,1)));
  h.guests.push(Guest{conn:1,placed_at:2,armor:[None,Some(ItemStack::new("test:a",1)),None,None],sent:RefCell::new(Vec::new())});
  tconstruct_haste_wire_send(&mut h,&mut TickCtx{world:&mut w});assert!(w.chunks.cache.contains_key(&(1,2)));
 }
}}
