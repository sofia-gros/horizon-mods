extern crate horizon_items;
extern crate horizon_world;
mod game {
 use horizon_items::items::ItemStack;
 use std::collections::{BTreeMap,HashSet};
 pub struct Health(pub f32); impl Health {pub fn health(&self)->f32{self.0}}
 pub struct Player {pub auto_spin_attack_dmg:f32,pub health:Health}
 pub struct Chunks {pub active:HashSet<Option<i32>>,pub raw:Option<Result<tconstruct_modifier_snapshot::Raw,String>>}
 impl Default for Chunks {fn default()->Self{Self{active:HashSet::new(),raw:Some(Ok((BTreeMap::from([("tconstruct:tinkering/modifiers/raging".into(),vec![tconstruct_modifier_snapshot::PINNED_RAGING.to_vec()])]),BTreeMap::new())))}}}
 impl Chunks {pub fn __horizon_mixins_tconstruct_modifier_resource_snapshot(&self)->&Option<Result<tconstruct_modifier_snapshot::Raw,String>>{&self.raw}pub fn __horizon_mixins_tconstruct_melee_active(&self)->&HashSet<Option<i32>>{&self.active}}
 #[derive(Default)] pub struct World {pub chunks:Chunks,pub levels:BTreeMap<String,i32>}
 pub struct Game<'a>{pub player:Option<Player>,pub lan_acting:Option<i32>,pub world:World,pub tool:Option<ItemStack>,pub marker:std::marker::PhantomData<&'a ()>}
 impl Game<'_>{pub fn held(&self)->Option<&ItemStack>{self.tool.as_ref()} pub fn attack_damage(&self)->f32{2.0}}
 pub mod tconstruct_source_material_traits {
  use super::*;pub struct Levels{pub levels:BTreeMap<String,i32>,pub durability_multiplier:f32}
  pub fn levels(world:&World,_:&ItemStack)->Option<Levels>{Some(Levels{levels:world.levels.clone(),durability_multiplier:1.0})}
  pub fn level_with_upgrade(stack:&ItemStack,traits:&Levels,id:&str)->Option<i32>{
   use horizon_world::nbt::Tag;
   let upgrades=stack.data()?.other("minecraft:custom_data")?.get("tic_upgrades");
   let amount=match upgrades {None=>0,Some(t @ Tag::Compound(_))=>match t.get(id){None=>0,Some(Tag::Int(n)) if *n>=0=>*n,_=>return None},_=>return None};
   let level=traits.levels.get(id).copied().unwrap_or(0);if level<0{return None;}level.checked_add(amount)
  }
 }
 pub mod tconstruct_source_tags {include!("../../mixins/tconstruct/source-tags.rs");}
 pub mod tconstruct_modifier_snapshot {include!("../../mixins/tconstruct/modifier-snapshot.rs");}
 pub mod tconstruct_source_ranged_wear { include!("../../mixins/tconstruct/source-ranged-wear.rs"); }
 pub mod tconstruct_source_station_stats { include!("../../mixins/tconstruct/source-station-stats.rs"); }
 pub mod tconstruct_source_material_effects { include!("../../mixins/tconstruct/source-material-effects.rs"); }
 include!("../../mixins/tconstruct/source-melee-damage-return.rs");
 #[test] fn acting_health_and_living_broken_guards(){
  use horizon_world::nbt::Tag;
  let mut tool=ItemStack::new("tconstruct:pickaxe",1);
  tool.data_mut().set_other("minecraft:custom_data",Some(Tag::compound([("tic_max_damage",Tag::Int(100)),("tic_raging_melee_multiplier",Tag::Float(1.0_f32.to_bits()))])));
  let mut game=Game{player:Some(Player{auto_spin_attack_dmg:0.0,health:Health(6.0)}),lan_acting:Some(1),world:World::default(),tool:Some(tool),marker:std::marker::PhantomData};
  game.world.levels.insert("tconstruct:raging".into(),1);
  let run=|g:&mut Game<'_>,living|tconstruct_melee_secondary_damage((2.0,1.0,false,false),g,living).0;
  assert!((run(&mut game,true)-3.25).abs()<0.0001);
  assert_eq!(run(&mut game,false),2.0);
  game.player.as_mut().unwrap().health.0=20.0;assert_eq!(run(&mut game,true),2.0);
  game.lan_acting=Some(5);game.player.as_mut().unwrap().health.0=1.0;assert!((run(&mut game,true)-4.5).abs()<0.0001);
  // Stored conditional multiplier (e.g. a definition factor plus Emerald)
  // scales the flat Raging contribution before cooldown and wear traits.
  game.tool.as_mut().unwrap().data_mut().set_other("minecraft:custom_data",Some(Tag::compound([("tic_max_damage",Tag::Int(100)),("tic_raging_melee_multiplier",Tag::Float(2.5_f32.to_bits()))])));
  assert!((run(&mut game,true)-8.25).abs()<0.0001);
  game.tool.as_mut().unwrap().data_mut().set_other("minecraft:custom_data",Some(Tag::compound([("tic_max_damage",Tag::Int(100)),("tic_raging_melee_multiplier",Tag::Float(1.0_f32.to_bits()))])));
  game.tool.as_mut().unwrap().damage=100;assert_eq!(run(&mut game,true),2.0);
  game.tool.as_mut().unwrap().damage=25;game.player.as_mut().unwrap().health.0=6.0;game.world.levels.insert("tconstruct:jagged".into(),1);
  assert!((run(&mut game,true)-3.28125).abs()<0.0001);
 }
}
