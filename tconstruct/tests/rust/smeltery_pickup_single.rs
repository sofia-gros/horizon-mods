pub use pluto_inventory_api as pluto_inventory;
#[path = "../../mixins/tconstruct/chest-inventory-api-lib.rs"]
mod tconstruct_inventory_api;
#[path = "../../mixins/tconstruct/source-smeltery-layout.rs"]
mod tconstruct_source_smeltery_layout;
#[path = "../../mixins/tconstruct/source-smeltery-page.rs"]
mod tconstruct_source_smeltery_page;
#[cfg(test)] mod tests {
 use super::tconstruct_source_smeltery_page::{prepare_single_insert, sent_page_matches};
 use horizon_items::items::ItemStack;
 use horizon_world::nbt::Tag;
 fn base() -> Tag { Tag::compound([("slots",Tag::List(vec![Tag::compound(Vec::<(&str,Tag)>::new()),horizon_items::item_nbt::item_to_nbt(&ItemStack::new("minecraft:gold_ore",1))])),("progress",Tag::IntArray(vec![37,88])),("opaque",Tag::Int(42))]) }
 #[test] fn insert_one_preserves_unrelated_raw_data() {
  let raw=base();let held=ItemStack::new("minecraft:iron_ore", 7);let (after,placed,remaining)=prepare_single_insert(&raw,0,&held).unwrap();
  assert_eq!((placed.count,remaining.count),(1,6));assert_eq!(after.get("opaque"),raw.get("opaque"));
  let (Tag::List(before),Tag::List(next))=(raw.get("slots").unwrap(),after.get("slots").unwrap()) else {panic!()};assert_eq!(before[1],next[1]);
  assert_eq!(after.get("progress"),Some(&Tag::IntArray(vec![0,88])));
 }
 #[test] fn unsent_guest_page_changes_are_rejected() {
  let mut sent = vec![None;90]; let mut expected = vec![None;54];
  assert!(sent_page_matches(&sent,&expected));
  expected[2]=Some(ItemStack::new("minecraft:iron_ore",1));
  assert!(!sent_page_matches(&sent,&expected));
  sent[2]=expected[2].clone(); assert!(sent_page_matches(&sent,&expected));
  assert!(!sent_page_matches(&sent[..53],&expected));
 }
 #[test] fn changed_stack_retaining_wire_is_rejected() {
  use horizon_items::item_data::Wire;
  use std::sync::Arc;
  let snapshot = ItemStack::new("minecraft:iron_ore",7);
  let mut changed=snapshot.clone();
  changed.data_mut().wire=Wire::new(snapshot, Arc::<[u8]>::from([1,2,3]));
  changed.damage=1;
  assert!(changed.wire_patch().is_none());
  assert!(changed.data().unwrap().wire.patch().is_some());
  assert!(prepare_single_insert(&base(),0,&changed).is_none());
  let mut plain=ItemStack::new("minecraft:iron_ore",7); plain.damage=1; plain.data_mut();
  assert_eq!(changed,plain); // Wire is deliberately omitted from PartialEq.
  let mut expected=vec![None;54]; let mut sent=vec![None;90];
  expected[2]=Some(plain); sent[2]=Some(changed);
  assert!(!sent_page_matches(&sent,&expected));
 }
 #[test] fn occupied_unknown_and_malformed_slots_are_never_overwritten() {
  let raw=base();let held=ItemStack::new("minecraft:iron_ore",7);
  assert!(prepare_single_insert(&raw,1,&held).is_none());assert!(prepare_single_insert(&raw,2,&held).is_none());
  let mut opaque=raw.clone();
  let Tag::Compound(fields)=&mut opaque else {panic!()};
  let Some(Tag::List(slots))=fields.get_mut(&"slots".into()) else {panic!()};
  let Tag::Compound(item)=&mut slots[1] else {panic!()}; item.insert("future".into(),Tag::Int(9));
  assert!(prepare_single_insert(&opaque,0,&held).is_none());
  let missing=Tag::compound([("slots",Tag::List(vec![Tag::compound(Vec::<(&str,Tag)>::new())]))]);assert!(prepare_single_insert(&missing,0,&held).is_none());
  assert!(prepare_single_insert(&raw,0,&ItemStack::new("minecraft:iron_ore",1)).is_none());
 }
 #[test] fn shared_projection_keeps_reserved_slots_layout_and_wire() {
  use horizon_items::item_data::Wire;
  use std::sync::Arc;
  let mut item=ItemStack::new("minecraft:iron_ore",7);
  item.data_mut().set_other("minecraft:custom_data",Some(Tag::Int(9)));
  let decoded=item.clone();item.data_mut().wire=Wire::new(decoded,Arc::<[u8]>::from([4,5,6]));
  let containers=[Some(item.clone()),Some(ItemStack::new("minecraft:bucket",1))];
  for count in [0,1,7,8,28,29,71] {
   let slots:Vec<_>=(0..count).map(|i|(i%3!=1).then(||item.clone())).collect();
   for offset in [0,8,999] {
    let mut expected=vec![None;54];expected[..2].clone_from_slice(&containers);
    for (menu_slot, cell) in expected.iter_mut().enumerate() {
     if let Some(index)=super::tconstruct_source_smeltery_layout::melting_index(slots.len(),offset,menu_slot) {*cell=slots[index].clone();}
    }
    let actual=super::tconstruct_source_smeltery_page::project_with_containers(&slots,offset,&containers);
    assert!(crate::pluto_stack_storage::inventory_snapshot_matches(&expected,Some(&actual)));
   }
  }
 }

 fn nested_held(field: usize, wire: Option<bool>) -> ItemStack {
  use horizon_items::item_data::Wire;
  use std::sync::Arc;
  let mut child=ItemStack::new("minecraft:brick",1);
  child.damage=2;
  child.data_mut().set_other("minecraft:custom_data",Some(Tag::compound([("future",Tag::Int(9))])));
  if let Some(stale)=wire {
   let mut decoded=child.clone();if stale {decoded.damage+=1;}
   child.data_mut().wire=Wire::new(decoded,Arc::<[u8]>::from([7,8,9]));
   assert_eq!(child.wire_patch().is_none(),stale);
  }
  let mut held=ItemStack::new("minecraft:shulker_box",7);
  match field {
   0=>held.data_mut().container.push((3,child)),
   1=>held.data_mut().charged.push(child),
   2=>held.data_mut().pot_decorations=Some(Box::new([None,Some(child),None,None])),
   3=>held.data_mut().bundle_contents.push(child),
   _=>unreachable!(),
  }
  held
 }

 #[test] fn nested_opaque_held_items_never_reach_single_insert_encoding() {
  for field in 0..4 {
   let plain=nested_held(field,None);
   let (saved,one,remainder)=prepare_single_insert(&base(),0,&plain).unwrap();
   assert_eq!((one.count,remainder.count),(1,6));
   assert_eq!(saved.get("opaque"),base().get("opaque"));
   for stale in [false,true] {
    let held=nested_held(field,Some(stale));let before=held.clone();let raw=base();
    assert!(held.same_item_same_components(&plain));
    assert!(prepare_single_insert(&raw,0,&held).is_none(),"field {field}, stale {stale}");
    assert!(crate::pluto_stack_storage::same_item_snapshot(&held,&before));
    assert_eq!(raw,base());
   }
  }
 }

 #[test] fn nested_opaque_sent_or_expected_pages_fail_closed() {
  for field in 0..4 {
   let mut plain=vec![None;54];plain[2]=Some(nested_held(field,None));
   assert!(sent_page_matches(&plain,&plain));
   for stale in [false,true] {
    let mut opaque=plain.clone();opaque[2]=Some(nested_held(field,Some(stale)));
    assert_eq!(plain,opaque,"native equality omits nested wire");
    assert!(!sent_page_matches(&opaque,&plain));
    assert!(!sent_page_matches(&plain,&opaque));
    assert!(!sent_page_matches(&opaque,&opaque));
   }
  }
 }
}
