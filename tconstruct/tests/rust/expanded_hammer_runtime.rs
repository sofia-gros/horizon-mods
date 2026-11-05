include!("ranged_upgrades.rs");
mod expanded_test {
 use super::*;use horizon_items::items::ItemStack;use horizon_world::nbt::Tag;use std::collections::{BTreeMap,HashMap,HashSet};
 fn tool(level:i32)->ItemStack {let mut s=ItemStack::new("tconstruct:sledge_hammer",1);s.data_mut().set_other("minecraft:custom_data",Some(Tag::compound([("tic_upgrades",Tag::compound([("tconstruct:expanded",Tag::Int(level))])),("unrelated",Tag::String("keep".into()))])));s}
 #[test] fn expanded_recipe_spends_ability_and_preserves_input_metadata() {
  let recipe=include_bytes!("../fixtures/expanded_recipe.json").to_vec();
  let recipes=BTreeMap::from([("expanded".into(),recipe)]);
  let tags=HashMap::from([("tconstruct:modifiable/aoe".into(),HashSet::from(["tconstruct:sledge_hammer".into(),"tconstruct:excavator".into()])),("forge:ingots/amethyst_bronze".into(),HashSet::from(["tconstruct:amethyst_bronze_ingot".into()])),("forge:slimeball/ichor".into(),HashSet::from(["tconstruct:ichor_slime_ball".into()]))]);
  let inputs=vec![Some(ItemStack::new("minecraft:piston",1)),Some(ItemStack::new("tconstruct:amethyst_bronze_ingot",1)),Some(ItemStack::new("minecraft:piston",1)),Some(ItemStack::new("tconstruct:ichor_slime_ball",1)),Some(ItemStack::new("tconstruct:ichor_slime_ball",1))];
  let hammer=tool(0);let before=hammer.clone();
  let plan=tconstruct_source_upgrade_rules::plan_with_traits(&recipes,&tags,&hammer,&inputs,2,1,&BTreeMap::new()).unwrap();assert_eq!(plan.amount,1);assert_eq!(plan.consumed.len(),6);assert_eq!(hammer,before);
  assert!(tconstruct_source_upgrade_rules::plan_with_traits(&recipes,&tags,&tool(1),&inputs,2,1,&BTreeMap::new()).is_none());
  assert_eq!(tconstruct_source_upgrade_rules::plan_with_traits(&recipes,&tags,&tool(1),&inputs,2,2,&BTreeMap::new()).unwrap().amount,2);
  assert!(tconstruct_source_upgrade_rules::plan_with_traits(&recipes,&tags,&tool(8),&inputs,2,20,&BTreeMap::new()).is_none());
  let mut other=tool(0);other.item="tconstruct:excavator".into();assert!(tconstruct_source_upgrade_rules::plan_with_traits(&recipes,&tags,&other,&inputs,2,20,&BTreeMap::new()).is_none());
  let mut bad=tool(0);bad.data_mut().set_other("minecraft:custom_data",Some(Tag::compound([("tic_upgrades",Tag::compound([("tconstruct:expanded",Tag::String("1".into()))]))])));
  assert!(tconstruct_source_upgrade_rules::plan_with_traits(&recipes,&tags,&bad,&inputs,2,20,&BTreeMap::new()).is_none());
  assert!(tconstruct_source_upgrade_rules::plan_with_traits(&recipes,&tags,&tool(-1),&inputs,2,20,&BTreeMap::new()).is_none());
 }
}
mod game {
 use horizon_items::items::ItemStack;use horizon_world::nbt::Tag;use std::collections::{BTreeMap,HashMap,HashSet};
 type Result<T>=std::result::Result<T,()>;
 pub(crate) use crate::tconstruct_source_upgrade_rules;
 pub mod tconstruct_source_area_shape {include!("../../mixins/tconstruct/source-pure-rules.rs");}
 pub mod tconstruct_source_chain_geometry {include!("../../mixins/tconstruct/source-chain-geometry.rs");}
 pub mod tconstruct_source_transform_geometry {include!("../../mixins/tconstruct/source-transform-geometry.rs");}
 pub mod tconstruct_source_ranged_wear {include!("../../mixins/tconstruct/source-ranged-wear.rs");}
 pub mod tconstruct_expanded {include!("../../mixins/tconstruct/source-expanded.rs");}
 pub mod tconstruct_world {pub type World=super::World;}
 pub mod tconstruct_source_material_traits {use super::*;pub struct Traits {pub levels:BTreeMap<String,i32>}pub fn levels(_: &World,_: &ItemStack)->Option<Traits>{Some(Traits {levels:BTreeMap::new()})}pub fn level_with_upgrade(stack:&ItemStack,_:&Traits,id:&str)->Option<i32>{match stack.data()?.other("minecraft:custom_data")?.get("tic_upgrades").and_then(|d|d.get(id)){None=>Some(0),Some(Tag::Int(n)) if *n>=0=>Some(*n),_=>None}}}
 pub mod tconstruct_source_mining_rules {pub fn harvestable(_: &horizon_world::nbt::Tag,_:&str,_:impl Fn(&str,&str)->bool)->Option<bool>{Some(true)}}
 #[derive(Default)]pub struct Raw(pub BTreeMap<String,Vec<Vec<u8>>>);
 #[derive(Default)]pub struct Chunks {pub tags:Option<std::result::Result<HashMap<String,HashSet<String>>,()>>,pub raw:Option<std::result::Result<Raw,()>>,pub defs:Option<BTreeMap<String,Vec<Vec<u8>>>>,pub active:HashSet<Option<u32>>,pub hits:HashMap<u32,([i32;3],[i32;3],HashSet<(i32,i32)>)>,pub spent:HashSet<Option<u32>>}
 impl Chunks {
 pub fn __horizon_mixins_tconstruct_resolved_item_tags(&self)->&Option<std::result::Result<HashMap<String,HashSet<String>>,()>>{&self.tags}
 pub fn __horizon_mixins_tconstruct_modifier_resource_snapshot(&self)->&Option<std::result::Result<Raw,()>>{&self.raw}
 pub fn __horizon_mixins_tconstruct_material_snapshot(&self)->&Option<BTreeMap<String,Vec<Vec<u8>>>>{&self.defs}
 pub fn __horizon_mixins_tconstruct_area_active(&self)->&HashSet<Option<u32>>{&self.active}
 pub fn __horizon_mixins_tconstruct_area_active_mut(&mut self)->&mut HashSet<Option<u32>>{&mut self.active}
 pub fn __horizon_mixins_tconstruct_area_guest_hits(&self)->&HashMap<u32,([i32;3],[i32;3],HashSet<(i32,i32)>)>{&self.hits}
 pub fn __horizon_mixins_tconstruct_area_guest_hits_mut(&mut self)->&mut HashMap<u32,([i32;3],[i32;3],HashSet<(i32,i32)>)>{&mut self.hits}
 pub fn __horizon_mixins_tconstruct_area_overwield_spent_mut(&mut self)->&mut HashSet<Option<u32>>{&mut self.spent}
 }
 pub struct World {pub chunks:Chunks,pub removed:HashSet<[i32;3]>}
 impl World {pub fn block(&self,x:i32,y:i32,z:i32)->u8 {if self.removed.contains(&[x,y,z]) {0}else{1}}}
 pub struct Fact {pub hardness:f32,pub name:Box<str>,pub can_occlude:bool}
 pub struct Res {pub facts:[Fact;2]}
 impl Res {pub fn facts(&self)->&Self{self}pub fn items(&self)->&Self{self}pub fn get(&self,state:u8)->&Fact{&self.facts[state as usize]}pub fn is_air(&self,state:u8)->bool{state==0}pub fn block_in_tag(&self,_:&str,_:&str)->bool{false}}
 pub struct Hit {pub pos:[i32;3],pub face:[i32;3]}
 pub struct Game<'a> {pub world:World,pub res:Res,pub remote:Option<()>,pub lan_acting:Option<u32>,pub yaw:f32,pub stack:ItemStack,pub face:[i32;3],pub fail:Option<[i32;3]>,pub break_after:Option<usize>,pub marker:std::marker::PhantomData<&'a ()>}
 impl Game<'_> {
 pub fn held(&self)->Option<&ItemStack>{Some(&self.stack)}pub fn pick(&self)->Option<Hit>{Some(Hit {pos:[0,0,0],face:self.face})}
 pub fn break_block_as_player(&mut self,pos:[i32;3])->Result<()> {
 if let std::ops::ControlFlow::Break(value)=tconstruct_area_break(self,pos) {return value;}
 if self.fail==Some(pos){return Err(());}self.world.removed.insert(pos);self.stack.damage+=1;
 if self.break_after==Some(self.world.removed.len()){self.stack.damage=1000;}Ok(())
 }
 }
 include!("../../mixins/tconstruct/source-area-break.rs");
 fn setup(level:i32)->Game<'static> {
 let mut stack=ItemStack::new("tconstruct:sledge_hammer",1);stack.data_mut().set_other("minecraft:custom_data",Some(Tag::compound([("tic_max_damage",Tag::Int(1000)),("tic_upgrades",Tag::compound([("tconstruct:expanded",Tag::Int(level))]))])));stack.data_mut().set_other("minecraft:tool",Some(Tag::compound(Vec::<(&str,Tag)>::new())));
 let mut chunks=Chunks::default();chunks.tags=Some(Ok(HashMap::from([("tconstruct:modifiable/aoe".into(),HashSet::from(["tconstruct:sledge_hammer".into()]))])));
 chunks.raw=Some(Ok(Raw(BTreeMap::from([("tconstruct:tinkering/modifiers/expanded".into(),vec![include_bytes!("../fixtures/expanded_modifier.json").to_vec()])]))));chunks.defs=Some(BTreeMap::from([("tconstruct:tinkering/tool_definitions/sledge_hammer".into(),vec![include_bytes!("../fixtures/expanded_hammer.json").to_vec()])]));
 Game {world:World {chunks,removed:HashSet::new()},res:Res {facts:[Fact {hardness:0.0,name:"minecraft:air".into(),can_occlude:false},Fact {hardness:1.0,name:"minecraft:stone".into(),can_occlude:true}]},remote:None,lan_acting:None,yaw:0.0,stack,face:[0,0,1],fail:None,break_after:None,marker:Default::default()}
 }
 #[test] fn real_area_hook_uses_expanded_source_dimensions_and_stops_when_broken() {
 for (level,count) in [(0,9),(1,15),(2,25),(8,121)] {let mut game=setup(level);game.break_block_as_player([0,0,0]).unwrap();assert_eq!(game.world.removed.len(),count);assert!(game.world.chunks.active.is_empty());}
 let mut game=setup(2);game.break_after=Some(2);game.break_block_as_player([0,0,0]).unwrap();assert_eq!(game.world.removed.len(),2);
 }
 #[test] fn guest_protection_and_failure_cleanup_use_actual_hook() {
 let mut game=setup(2);game.lan_acting=Some(7);game.world.chunks.hits.insert(7,([0,0,0],[0,0,1],HashSet::from([(-2,0)])));
 game.break_block_as_player([0,0,0]).unwrap();assert_eq!(game.world.removed.len(),20);assert!(!game.world.removed.iter().any(|p|p[0]==-2));assert!(game.world.chunks.hits.is_empty());
 let mut game=setup(2);game.fail=Some([-2,-2,0]);assert!(game.break_block_as_player([0,0,0]).is_err());assert!(game.world.chunks.active.is_empty());
 }
 #[test] fn changed_source_tag_invalid_level_and_remote_do_not_expand() {
 for mode in 0..5 {let mut game=setup(if mode==0 {9}else{2});match mode {1=>game.world.chunks.raw.as_mut().unwrap().as_mut().unwrap().0.clear(),2=>game.world.chunks.tags.as_mut().unwrap().as_mut().unwrap().clear(),3=>game.remote=Some(()),4=>game.world.chunks.defs.as_mut().unwrap().clear(),_=>{}};game.break_block_as_player([0,0,0]).unwrap();assert_eq!(game.world.removed.len(),1);}
 }
}
