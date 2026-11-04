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
}
