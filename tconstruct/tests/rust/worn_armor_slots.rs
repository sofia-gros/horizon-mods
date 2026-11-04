// Lightweight boundary doubles execute the real equipment helper. This is not
// a generated-host or gameplay test.
extern crate self as horizon_world;
use std::collections::{BTreeMap, HashMap, HashSet};
pub mod nbt {
    use super::BTreeMap;
    #[derive(Clone)] pub enum Tag { Float(u32), Int(i32), Compound(BTreeMap<String, Tag>) }
    impl Tag { pub fn get(&self, key: &str) -> Option<&Self> { match self { Self::Compound(v) => v.get(key), _ => None } } }
}
use nbt::Tag;
#[derive(Clone)] pub struct Data(Tag);
impl Data { fn other(&self, key: &str) -> Option<&Tag> { (key == "minecraft:custom_data").then_some(&self.0) } }
#[derive(Clone)] pub struct Stack { item: Box<str>, data: Option<Data>, broken: bool }
impl Stack { fn data(&self) -> Option<&Data> { self.data.as_ref() } }
type Tags = HashMap<String, HashSet<String>>;
pub struct Chunks(Option<Result<Tags, ()>>);
impl Chunks {
    #[allow(non_snake_case)] fn __horizon_mixins_tconstruct_resolved_item_tags(&self) -> &Option<Result<Tags, ()>> { &self.0 }
}
pub struct World { chunks: Chunks }
#[derive(Default)] pub struct Player { values: BTreeMap<(String, String), (f64, String)> }
impl Player {
    fn set_modifier(&mut self, attribute: &str, id: &str, amount: f64, op: &str, enabled: bool) -> Result<(), ()> {
        if enabled { self.values.insert((attribute.into(), id.into()), (amount, op.into())); }
        else { self.values.remove(&(attribute.into(), id.into())); }
        Ok(())
    }
}
pub mod game {
    use super::*;
    pub struct Game<'a> {
        pub armor: [Option<Stack>; 4], pub world: World, pub player: Option<Player>,
        pub applied_modifiers: Vec<(String, String)>, pub marker: std::marker::PhantomData<&'a ()>,
    }
    pub mod tconstruct_source_ranged_wear { pub fn is_broken(s: &super::Stack) -> bool { s.broken } }
    pub mod tconstruct_source_material_traits {
        pub struct Traits { pub levels: super::BTreeMap<String, i32> }
        pub fn levels(_: &super::World, _: &super::Stack) -> Option<Traits> { None }
    }
}
include!("../../mixins/tconstruct/material-armor-attributes.rs");
fn stack(id: &str) -> Stack {
    Stack { item: id.into(), data: Some(Data(Tag::Compound([
        ("tic_armor".into(), Tag::Float(3.0f32.to_bits())),
        ("tic_armor_toughness".into(), Tag::Float(2.0f32.to_bits())),
        ("tic_knockback_resistance".into(), Tag::Float(0.1f32.to_bits())),
    ].into()))), broken: false }
}
fn game() -> game::Game<'static> {
 game::Game {armor:[None,None,None,None],world:World {chunks:Chunks(None)},player:Some(Player::default()),applied_modifiers:vec![],marker:std::marker::PhantomData}
}
fn apply(g:&mut game::Game<'_>) {
 // Model the existing apply_equipment clearing contract before calling the real helper.
 for (attr,id) in std::mem::take(&mut g.applied_modifiers) {
  g.player.as_mut().unwrap().set_modifier(&attr,&id,0.0,"add_value",false).unwrap();
 }
 tconstruct_material_armor(g);
}
#[test] fn all_stock_armor_classes_only_grant_base_stats_in_their_slot() {
 let families=[
 (0,vec!["travelers_helmet","plate_helmet","slime_helmet"]),
 (1,vec!["travelers_chestplate","plate_chestplate","slimy_chestplate","slime_wings"]),
 (2,vec!["travelers_leggings","plate_leggings","slime_leggings"]),
 (3,vec!["travelers_boots","plate_boots","slime_boots"])];
 for (correct,ids) in families {for id in ids {for actual in 0..4 {
  let mut g=game();g.armor[actual]=Some(stack(&format!("tconstruct:{id}")));apply(&mut g);
  assert_eq!(g.applied_modifiers.len(),if actual==correct {3}else{0},"{id} slot {actual}");
 }}}
}
#[test] fn shields_staffs_unknown_and_external_items_do_not_grant_worn_base_stats() {
 for id in ["tconstruct:plate_shield","tconstruct:travelers_shield","tconstruct:battlesign","tconstruct:sky_staff","tconstruct:future_helmet","external:helmet"] {
  let mut g=game();g.armor[0]=Some(stack(id));
  // A mutable aggregate tag cannot change pinned class eligibility.
  g.world.chunks.0=Some(Ok(HashMap::from([("tconstruct:modifiable/armor".into(),HashSet::from([id.into()]))])));
  apply(&mut g);assert!(g.applied_modifiers.is_empty(),"{id}");
 }
}
#[test] fn wrong_slot_transition_removes_base_stats_and_repair_restores() {
 let mut g=game();g.armor[0]=Some(stack("tconstruct:plate_helmet"));apply(&mut g);assert_eq!(g.applied_modifiers.len(),3);
 g.armor[3]=g.armor[0].take();apply(&mut g);assert!(g.player.as_ref().unwrap().values.is_empty());
 g.armor[0]=g.armor[3].take();g.armor[0].as_mut().unwrap().broken=true;apply(&mut g);assert!(g.applied_modifiers.is_empty());
 g.armor[0].as_mut().unwrap().broken=false;apply(&mut g);assert_eq!(g.applied_modifiers.len(),3);
}
#[test] fn wrong_slot_attributes_stop_but_equipment_change_gravity_remains() {
 for broken in [false,true] {
  let mut g=game();let mut item=stack("tconstruct:plate_helmet");item.broken=broken;
  let Some(Data(Tag::Compound(ref mut data)))=item.data else{panic!()};
  data.insert("tic_upgrades".into(),Tag::Compound([("tconstruct:vintage".into(),Tag::Int(1)),("tconstruct:step_up".into(),Tag::Int(1))].into()));
  g.armor[3]=Some(item);apply(&mut g);
  let values=&g.player.as_ref().unwrap().values;
  assert_eq!(values.len(),1);
  assert!(values.contains_key(&("minecraft:gravity".into(),"tconstruct:vintage_gravity/feet".into())));
  assert!(!values.contains_key(&("minecraft:step_height".into(),"tconstruct:step_up/feet".into())));
  assert!(!values.keys().any(|(_,id)|id.starts_with("tconstruct:tic_")));
 }
}

#[test] fn each_supported_attribute_requires_correct_slot_and_intact_item() {
 for (id,amount,attribute,op) in [
  ("vintage",-0.1,"movement_speed","add_multiplied_total"),
  ("heavy",-0.1,"movement_speed","add_multiplied_base"),
  ("solid",-0.1,"movement_speed","add_multiplied_base"),
  ("featherweight",0.05,"movement_speed","add_multiplied_base"),
  ("entwined",0.1,"movement_speed","add_multiplied_total"),
  ("speedy",0.1,"movement_speed","add_multiplied_total"),
  ("step_up",0.5,"step_height","add_value"),
 ] {
  for slot in [0,3] {for broken in [false,true] {
   let mut g=game();let mut item=stack("tconstruct:plate_helmet");item.broken=broken;
   let Some(Data(Tag::Compound(ref mut data)))=item.data else {panic!()};
   data.insert("tic_upgrades".into(),Tag::Compound([(format!("tconstruct:{id}"),Tag::Int(if id=="speedy" {45}else{1}))].into()));
   g.world.chunks.0=Some(Ok(HashMap::from([("tconstruct:modifiable/armor".into(),HashSet::from(["tconstruct:plate_helmet".into()]))])));
   g.armor[slot]=Some(item);apply(&mut g);
   let values=&g.player.as_ref().unwrap().values;
   let entry=values.iter().find(|((attr,_),_)|attr==&format!("minecraft:{attribute}"));
   if slot==0 && !broken {assert_eq!(entry.map(|(_,value)|value),Some(&(amount,op.into())),"{id}");}
   else {assert!(entry.is_none(),"{id} slot {slot} broken {broken}");}
   let gravity=values.keys().any(|(attr,_)|attr=="minecraft:gravity");
   assert_eq!(gravity,matches!(id,"vintage"|"heavy"|"solid"));
  }}
 }
}
#[test] fn moving_to_wrong_slot_clears_movement_and_step_but_keeps_gravity() {
 let mut g=game();let mut item=stack("tconstruct:plate_helmet");
 let Some(Data(Tag::Compound(ref mut data)))=item.data else {panic!()};
 data.insert("tic_upgrades".into(),Tag::Compound([("tconstruct:vintage".into(),Tag::Int(1)),("tconstruct:step_up".into(),Tag::Int(1))].into()));
 g.armor[0]=Some(item);apply(&mut g);assert_eq!(g.applied_modifiers.len(),6);
 g.armor[3]=g.armor[0].take();apply(&mut g);assert_eq!(g.applied_modifiers.len(),1);
 assert_eq!(g.applied_modifiers[0],("minecraft:gravity".into(),"tconstruct:vintage_gravity/feet".into()));
}
#[test] fn unsupported_classes_and_missing_featherweight_tag_fail_closed() {
 for id in ["tconstruct:plate_shield","tconstruct:sky_staff","tconstruct:future_helmet"] {
  let mut g=game();let mut item=stack(id);
  let Some(Data(Tag::Compound(ref mut data)))=item.data else {panic!()};
  data.insert("tic_upgrades".into(),Tag::Compound([("tconstruct:vintage".into(),Tag::Int(1))].into()));
  g.armor[0]=Some(item);apply(&mut g);
  assert_eq!(g.applied_modifiers,vec![("minecraft:gravity".into(),"tconstruct:vintage_gravity/head".into())]);
 }
 let mut g=game();let mut item=stack("tconstruct:plate_helmet");
 let Some(Data(Tag::Compound(ref mut data)))=item.data else {panic!()};
 data.insert("tic_upgrades".into(),Tag::Compound([("tconstruct:featherweight".into(),Tag::Int(1))].into()));
 g.armor[0]=Some(item);apply(&mut g);
 assert_eq!(g.applied_modifiers.len(),3);
 assert!(!g.applied_modifiers.iter().any(|(attr,_)|attr=="minecraft:movement_speed"));
}

#[test] fn base_defense_stats_require_positive_saved_floats_individually() {
 for (key,attr) in [("tic_armor","armor"),("tic_armor_toughness","armor_toughness"),("tic_knockback_resistance","knockback_resistance")] {
  for amount in [0.0f32,-0.0,-1.0,f32::NEG_INFINITY,f32::NAN,f32::from_bits(1),0.25,f32::INFINITY] {
   let mut g=game();let mut item=stack("tconstruct:plate_helmet");
   let Some(Data(Tag::Compound(ref mut data)))=item.data else {panic!()};
   data.insert(key.into(),Tag::Float(amount.to_bits()));g.armor[0]=Some(item);apply(&mut g);
   let modifier=g.player.as_ref().unwrap().values.get(&(format!("minecraft:{attr}"),format!("tconstruct:{key}/head")));
   if amount>0.0 {assert_eq!(modifier,Some(&(f64::from(amount),"add_value".into())));}
   else {assert!(modifier.is_none(),"{key} {amount:?}");}
   assert_eq!(g.applied_modifiers.len(),if amount>0.0 {3}else{2});
  }
 }
}
#[test] fn invalidated_base_stat_is_removed_and_other_modifiers_preserved() {
 let mut g=game();let mut item=stack("tconstruct:plate_helmet");
 let Some(Data(Tag::Compound(ref mut data)))=item.data else {panic!()};
 data.insert("tic_upgrades".into(),Tag::Compound([("tconstruct:vintage".into(),Tag::Int(1))].into()));
 g.armor[0]=Some(item);apply(&mut g);assert_eq!(g.applied_modifiers.len(),5);
 let Some(Data(Tag::Compound(ref mut data)))=g.armor[0].as_mut().unwrap().data else {panic!()};
 data.insert("tic_armor".into(),Tag::Float(f32::NAN.to_bits()));
 apply(&mut g);assert_eq!(g.applied_modifiers.len(),4);
 assert!(!g.applied_modifiers.iter().any(|(_,id)|id=="tconstruct:tic_armor/head"));
 assert!(g.applied_modifiers.iter().any(|(_,id)|id=="tconstruct:vintage_movement/head"));
 assert!(g.applied_modifiers.iter().any(|(_,id)|id=="tconstruct:vintage_gravity/head"));
}
