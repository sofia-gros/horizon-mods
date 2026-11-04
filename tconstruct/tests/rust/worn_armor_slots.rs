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
pub struct World { chunks: Chunks, traits: BTreeMap<String,i32> }
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
        pub fn levels(world: &super::World, _: &super::Stack) -> Option<Traits> { Some(Traits { levels:world.traits.clone() }) }
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
 game::Game {armor:[None,None,None,None],world:World {chunks:Chunks(None),traits:BTreeMap::new()},player:Some(Player::default()),applied_modifiers:vec![],marker:std::marker::PhantomData}
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
   if slot==0 && !broken {let expected=f64::from(amount as f32);assert_eq!(entry.map(|(_,value)|value),Some(&(expected,op.into())),"{id}");}
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

#[test] fn integer_modifier_formulas_compute_float_then_widen() {
 for (id,coefficient,attribute,op,gravity) in [
  ("vintage",-0.1f32,"movement_speed","add_multiplied_total",Some(0.1f32)),
  ("heavy",-0.1,"movement_speed","add_multiplied_base",Some(0.05)),
  ("solid",-0.1,"movement_speed","add_multiplied_base",Some(0.05)),
  ("featherweight",0.05,"movement_speed","add_multiplied_base",None),
  ("entwined",0.1,"movement_speed","add_multiplied_total",None),
  ("step_up",0.5,"step_height","add_value",None),
 ] {
  for level in [1,3,16_777_217] {
   let mut g=game();let mut item=stack("tconstruct:plate_helmet");
   let Some(Data(Tag::Compound(ref mut data)))=item.data else {panic!()};
   data.insert("tic_upgrades".into(),Tag::Compound([(format!("tconstruct:{id}"),Tag::Int(level))].into()));
   g.world.chunks.0=Some(Ok(HashMap::from([("tconstruct:modifiable/armor".into(),HashSet::from(["tconstruct:plate_helmet".into()]))])));
   g.armor[0]=Some(item);apply(&mut g);
   let values=&g.player.as_ref().unwrap().values;
   let (_, (amount,operation))=values.iter().find(|((attr,_),_)|attr==&format!("minecraft:{attribute}")).unwrap();
   let expected=f64::from(0.0f32+coefficient*level as f32);
   assert_eq!(amount.to_bits(),expected.to_bits(),"{id} level {level}");
   assert_eq!(operation,op);
   if let Some(coefficient)=gravity {
    let (_, (amount,operation))=values.iter().find(|((attr,_),_)|attr=="minecraft:gravity").unwrap();
    assert_eq!(amount.to_bits(),f64::from(0.0f32+coefficient*level as f32).to_bits());
    assert_eq!(operation,"add_multiplied_total");
   }
  }
 }
 // Counterexamples to the former double-only calculations.
 assert_ne!(f64::from(0.1f32).to_bits(),0.1f64.to_bits());
 assert_ne!(f64::from(0.5f32*16_777_217i32 as f32).to_bits(),(0.5f64*16_777_217.0).to_bits());
}

#[test] fn speedy_merges_logical_levels_before_incremental_float_rounding() {
 for trait_level in [0,1,3,16_777_217] {
  for progress in [0,1,22,44,45,46,89,90] {
   let mut g=game();let mut item=stack("tconstruct:plate_helmet");
   let Some(Data(Tag::Compound(ref mut data)))=item.data else {panic!()};
   data.insert("tic_upgrades".into(),Tag::Compound([("tconstruct:speedy".into(),Tag::Int(progress))].into()));
   g.world.traits.insert("tconstruct:speedy".into(),trait_level);
   g.armor[0]=Some(item);apply(&mut g);
   let remainder=progress%45;
   let logical=trait_level+progress/45+i32::from(remainder!=0);
   let effective=if remainder==0 {logical as f32}else{logical as f32+(remainder-45) as f32/45.0f32};
   let expected=f64::from(0.0f32+0.1f32*effective);
   let value=g.player.as_ref().unwrap().values.get(&("minecraft:movement_speed".into(),"tconstruct:speedy_movement/head".into()));
   if effective>0.0 {assert_eq!(value,Some(&(expected,"add_multiplied_total".into())));}
   else {assert!(value.is_none());}
  }
 }
 let source=f64::from(0.1f32*(1.0f32+(-44.0f32/45.0)));
 let naive_float=f64::from(0.1f32*(1.0f32/45.0));
 assert_ne!(source.to_bits(),naive_float.to_bits());
 assert_ne!(source.to_bits(),(0.1f64/45.0).to_bits());
 let merged=16_777_218i32 as f32+(-44.0f32/45.0);
 let separately_rounded=16_777_217i32 as f32+(1.0f32+(-44.0f32/45.0));
 assert_ne!((0.1f32*merged).to_bits(),(0.1f32*separately_rounded).to_bits());
}
#[test] fn speedy_overflow_is_local_and_gates_remain_intact() {
 for (trait_level,progress,slot,broken) in [(i32::MAX,45,0,false),(i32::MAX,1,0,false),(1,1,3,false),(1,1,0,true)] {
  let mut g=game();let mut item=stack("tconstruct:plate_helmet");item.broken=broken;
  let Some(Data(Tag::Compound(ref mut data)))=item.data else {panic!()};
  data.insert("tic_upgrades".into(),Tag::Compound([("tconstruct:speedy".into(),Tag::Int(progress)),("tconstruct:vintage".into(),Tag::Int(1))].into()));
  g.world.traits.insert("tconstruct:speedy".into(),trait_level);
  g.armor[slot]=Some(item);apply(&mut g);
  assert!(!g.applied_modifiers.iter().any(|(_,id)|id.starts_with("tconstruct:speedy_")));
  assert!(g.applied_modifiers.iter().any(|(_,id)|id.starts_with("tconstruct:vintage_gravity/")));
 }
}
#[test] fn speedy_existing_missing_malformed_and_negative_fallbacks_remain() {
 for trait_level in [-1,0,1] {
  for progress in [None,Some(Tag::Int(-1)),Some(Tag::Float(1.0f32.to_bits()))] {
   let mut g=game();let mut item=stack("tconstruct:plate_helmet");
   let Some(Data(Tag::Compound(ref mut data)))=item.data else {panic!()};
   if let Some(progress)=progress {
    data.insert("tic_upgrades".into(),Tag::Compound([("tconstruct:speedy".into(),progress)].into()));
   }
   g.world.traits.insert("tconstruct:speedy".into(),trait_level);g.armor[0]=Some(item);apply(&mut g);
   let value=g.player.as_ref().unwrap().values.get(&("minecraft:movement_speed".into(),"tconstruct:speedy_movement/head".into()));
   if trait_level>0 {assert_eq!(value,Some(&(f64::from(0.1f32),"add_multiplied_total".into())));}
   else {assert!(value.is_none());}
  }
 }
}
