// Appended to the real-store fixture by the pinned runner. Only host world,
// recipe lookup and portable conversion are mocked; interaction is extracted
// verbatim, and state/store/item codecs are production implementations.
use horizon_items::items::ItemStack;
use std::collections::{HashMap,HashSet};
type Pos=(i32,i32,i32);
#[derive(Clone)] enum BlockEntity { Chest(Vec<Option<ItemStack>>) }
#[derive(Default)] struct Chunks(HashMap<(i32,i32),std::sync::Arc<tconstruct_world::Chunk>>);
impl std::ops::Deref for Chunks {type Target=HashMap<(i32,i32),std::sync::Arc<tconstruct_world::Chunk>>;fn deref(&self)->&Self::Target{&self.0}}
impl std::ops::DerefMut for Chunks {fn deref_mut(&mut self)->&mut Self::Target{&mut self.0}}
impl Chunks {
 fn __horizon_mixins_tconstruct_tank_fluid_context(&self)->Option<&Result<(HashSet<String>,()),()>>{None}
 fn __horizon_mixins_tconstruct_resolved_item_tags(&self)->Option<&Result<HashMap<String,HashSet<String>>,()>>{None}
}
const CASTING_BLOCKS:[&str;4]=["tconstruct:seared_table","tconstruct:scorched_table","tconstruct:seared_basin","tconstruct:scorched_basin"];
struct Facts;struct Fact {name:String}impl Facts{fn get(&self,id:u8)->Fact{Fact{name:CASTING_BLOCKS.get(id as usize).unwrap_or(&"minecraft:air").to_string()}}}
struct Resources;impl Resources{fn items(&self)->HashMap<String,()>{HashMap::new()}fn facts(&self)->Facts{Facts}}
struct Stats;impl Stats{fn add(&mut self,_:&str,_:&str,_:u32){}}
struct Game<'a>{world:tconstruct_world::World,held:Option<ItemStack>,given:Vec<ItemStack>,creative:bool,lan:Option<()>,lan_acting_uuid:Option<()>,res:Resources,stats:Stats,fault:u8,marker:std::marker::PhantomData<&'a ()>}
impl Game<'_>{
 fn held(&self)->Option<&ItemStack>{self.held.as_ref()}fn held_mut(&mut self)->&mut Option<ItemStack>{&mut self.held}
 fn ensure_datapacks(&mut self){
  let pos=(1,70,1);let kind=if self.world.block(1,70,1)<2{"tconstruct:table"}else{"tconstruct:basin"};
  match self.fault {
   1=>self.held=Some(ItemStack::new("minecraft:stone",1)),
   2=>if let Some(BlockEntity::Chest(slots))=self.world.block_entities.get_mut(&pos){slots[0]=Some(ItemStack::new("minecraft:diamond",1));},
   3=>{let mut raw=tconstruct_source_melter_store::read(&self.world,pos,kind).unwrap();if let horizon_world::nbt::Tag::Compound(fields)=&mut raw{fields.insert("future".into(),horizon_world::nbt::Tag::Int(999));}replace_casting_raw(self,pos,kind,raw);},
   4=>{self.world.blocks.insert(pos,99);},
   5=>self.world.chunks.clear(),
   6=>self.creative=!self.creative,
   7=>self.held.as_mut().unwrap().count+=1,
   8=>{let item=self.held.as_mut().unwrap();item.data_mut().wire=horizon_items::item_data::Wire::new(item.clone(),std::sync::Arc::<[u8]>::from([1,2,3]));item.damage=1;},
   9=>if let Some(BlockEntity::Chest(slots))=self.world.block_entities.get_mut(&pos){slots[1]=Some(ItemStack::new("minecraft:glass",1));},
   _=>{},
  }
 }fn give_or_drop(&mut self,s:ItemStack){self.given.push(s)}fn native_acting_id(&self)->u8{0}
 fn native_item_crafted(&mut self,_:u8,_:&str,_:&ItemStack){}
}
impl tconstruct_world::World{fn block(&self,x:i32,y:i32,z:i32)->u8{self.blocks.get(&(x,y,z)).copied().unwrap_or(0)}}
mod tconstruct_source_fluid_container {
 use super::*;#[derive(Clone,Copy)]pub enum Direction{EmptyItem,Auto}
 pub fn casting_portable_rewrite_safe(_: &ItemStack)->bool{true}
 pub fn transfer_single_with_color(tank: &mut tconstruct_source_melter_state::Tank,slots:&mut [Option<ItemStack>],direction:Direction,_:&impl Fn(&str)->bool,_:&impl Fn(&str)->bool,_:Option<&HashMap<String,HashSet<String>>>,_:bool)->bool{
  let Some(mut held)=slots[0].clone().filter(|item|item.count>0)else{return false;};
  let result=match &*held.item {
   "minecraft:water_bucket" if tank.insert("minecraft:water",1000)=>"minecraft:bucket",
   "minecraft:bucket" if matches!(direction,Direction::Auto) && tank.fluid.as_deref()==Some("minecraft:water") && tank.take(1000).is_some()=>"minecraft:water_bucket",
   _=>return false,
  };
  held.count-=1;slots[0]=(held.count>0).then_some(held);slots[1]=Some(ItemStack::new(result,1));true
 }
}
mod tconstruct_source_melter_world {
 use super::*;pub struct Recipe{pub result:String,pub pattern_consumed:bool}
 pub fn molding(_: &tconstruct_world::World,_:bool,material:&str,pattern:Option<&str>)->Option<(String,Recipe)>{
  (material=="minecraft:sand" && matches!(pattern,Some("tconstruct:pattern"|"tconstruct:consumed_pattern"))).then(||("test:mold".into(),Recipe{result:"minecraft:glass".into(),pattern_consumed:pattern==Some("tconstruct:consumed_pattern")}))
 }
}
mod interaction {
 use super::*;use tconstruct_source_melter_state::{Casting,Tank};use tconstruct_source_melter_store as store;
 fn ensure(_: &mut Game<'_>,_:Pos,_:&str)->bool{true}
 fn kind(block: &str)->Option<&'static str>{match block{"tconstruct:seared_table"|"tconstruct:scorched_table"=>Some("tconstruct:table"),"tconstruct:seared_basin"|"tconstruct:scorched_basin"=>Some("tconstruct:basin"),_=>None}}
 struct Fluid{amount:u32}struct Recipe{fluid:Fluid,cooling_time:u32,consumes_cast:bool,switch_slots:bool}
 fn casting_recipe_at(_: &Game<'_>,_:Pos,_:&str,_:&Casting,_:&Tank)->Option<(String,Recipe,ItemStack)>{None}
 // __CASTING_AT__
 // __INTERACT_CASTING__
}
fn interaction_game(input:Option<ItemStack>,held:Option<ItemStack>)->(Game<'static>,Pos){
 let mut state=tconstruct_source_melter_state::Casting::new();state.input=input.clone();
 let(world,pos)=casting_world(state.to_nbt());let mut game=Game{world,held,given:vec![],creative:false,lan:None,lan_acting_uuid:None,res:Resources,stats:Stats,fault:0,marker:std::marker::PhantomData};
 game.world.block_entities.insert(pos,BlockEntity::Chest(vec![input,None]));(game,pos)
}
#[test]
fn actual_casting_interaction_rejects_wire_insertion_molding_and_live_slots(){
 use horizon_items::item_data::Wire;use std::sync::Arc;
 for stale in [false,true]{for mode in 0..3{
  let molding=mode>0;
  let mut item=ItemStack::new(if mode==2{"tconstruct:consumed_pattern"}else if molding{"tconstruct:pattern"}else{"minecraft:stone"},2);
  item.data_mut().wire=Wire::new(item.clone(),Arc::<[u8]>::from([1,2,3]));if stale{item.damage=1;}
  assert_eq!(item.wire_patch().is_none(),stale);
  let (mut game,pos)=interaction_game(molding.then(||ItemStack::new("minecraft:sand",1)),Some(item));
  let before=tconstruct_source_melter_store::read(&game.world,pos,"tconstruct:table");let held=game.held.clone();
  assert!(interaction::interact_casting(&mut game,pos,"tconstruct:seared_table"));
  assert_eq!(game.held,held);assert!(game.held.as_ref().unwrap().data().unwrap().wire.patch().is_some());
  assert_eq!(tconstruct_source_melter_store::read(&game.world,pos,"tconstruct:table"),before);assert!(game.world.modified.is_empty());assert!(game.given.is_empty());
  // Live chest state may differ from its persisted decoded counterpart.
  let opaque=game.held.clone().unwrap();game.held=None;
  if let Some(BlockEntity::Chest(slots))=game.world.block_entities.get_mut(&pos){slots[0]=Some(opaque);}
  assert!(!interaction::interact_casting(&mut game,pos,"tconstruct:seared_table"));
  assert_eq!(tconstruct_source_melter_store::read(&game.world,pos,"tconstruct:table"),before);assert!(game.world.modified.is_empty());assert!(game.given.is_empty());
 }}
}
#[test]
fn actual_casting_interaction_accepts_normal_insertion_and_reusable_molding(){
 for molding in [false,true]{
 let held=ItemStack::new(if molding{"tconstruct:pattern"}else{"minecraft:stone"},2);
 let (mut game,pos)=interaction_game(molding.then(||ItemStack::new("minecraft:sand",1)),Some(held));
 assert!(interaction::interact_casting(&mut game,pos,"tconstruct:seared_table"));
 assert_eq!(game.held.unwrap().count,1);
 let raw=tconstruct_source_melter_store::read(&game.world,pos,"tconstruct:table").unwrap();let state=tconstruct_source_melter_state::Casting::from_nbt(&raw).unwrap();
 assert_eq!(&*state.input.unwrap().item,if molding{"minecraft:glass"}else{"minecraft:stone"});
 assert_eq!(state.output.is_some(),molding);assert!(!game.world.modified.is_empty());
 }
}

fn replace_casting_raw(game:&mut Game<'_>,pos:Pos,kind:&str,raw:horizon_world::nbt::Tag){
 use horizon_world::nbt::Tag;use std::sync::Arc;
 let chunk=Arc::make_mut(game.world.chunks.get_mut(&(pos.0>>4,pos.2>>4)).unwrap());
 let saved=Arc::make_mut(chunk.saved.as_mut().unwrap());
 saved.block_entity_sources[0]=(pos,Tag::compound([("id",Tag::String(kind.into())),(kind,raw)]));
}
fn portable_casting_game(block:u8,filled:bool,count:u32)->(Game<'static>,Pos,&'static str){
 use horizon_world::nbt::Tag;
 let input=ItemStack::new("tconstruct:ingot_cast",1);
 let (mut game,pos)=interaction_game(Some(input.clone()),Some(ItemStack::new(if filled{"minecraft:bucket"}else{"minecraft:water_bucket"},count)));
 game.world.blocks.insert(pos,block);let kind=if block<2{"tconstruct:table"}else{"tconstruct:basin"};
 let mut state=tconstruct_source_melter_state::Casting::new();state.input=Some(input);
 assert!(state.prepare("test:water",1000,2,ItemStack::new("minecraft:ice",1),false,false));
 if filled{assert!(state.tank.insert("minecraft:water",1000));}
 let mut raw=state.to_nbt();
 if let Tag::Compound(fields)=&mut raw {
  fields.insert("future".into(),Tag::Int(7));
  if let Some(Tag::Compound(tank))=fields.get_mut(&"tank".into()){tank.insert("future_tank".into(),Tag::Int(8));}
  if let Some(Tag::Compound(input))=fields.get_mut(&"input".into()){input.insert("future_item".into(),Tag::Int(9));}
 }
 replace_casting_raw(&mut game,pos,kind,raw);(game,pos,kind)
}
#[test]
fn actual_casting_held_transfers_preserve_extensions_in_all_four_blocks(){
 use horizon_world::nbt::Tag;
 for block in 0..4{for filled in [false,true]{for count in [1,2]{for creative in [false,true]{
  let(mut game,pos,kind)=portable_casting_game(block,filled,count);game.creative=creative;
  let held=game.held.clone();let before=tconstruct_source_melter_store::read(&game.world,pos,kind).unwrap();
  assert!(interaction::interact_casting(&mut game,pos,CASTING_BLOCKS[block as usize]));
  let raw=tconstruct_source_melter_store::read(&game.world,pos,kind).unwrap();let state=tconstruct_source_melter_state::Casting::from_nbt(&raw).unwrap();
  assert_eq!(state.tank.amount,if filled{0}else{1000});assert_eq!(state.recipe.is_none(),filled);
  assert_eq!(raw.get("future"),Some(&Tag::Int(7)));assert_eq!(raw.get("tank").unwrap().get("future_tank"),Some(&Tag::Int(8)));
  assert_eq!(raw.get("input"),before.get("input"));assert!(!game.world.modified.is_empty());
  let result=if filled{"minecraft:water_bucket"}else{"minecraft:bucket"};
  if creative{assert_eq!(game.held,held);assert_eq!(game.given.len(),usize::from(filled));}
  else if count==1{assert_eq!(&*game.held.as_ref().unwrap().item,result);assert!(game.given.is_empty());}
  else{assert_eq!(game.held.as_ref().unwrap().count,count-1);assert_eq!(game.given.len(),1);assert_eq!(&*game.given[0].item,result);}
 }}}}
}
#[test]
fn actual_casting_held_transfer_rechecks_live_saved_block_and_player_snapshots(){
 for fault in 1..=9{let(mut game,pos,kind)=portable_casting_game(0,false,1);game.fault=fault;
  assert!(interaction::interact_casting(&mut game,pos,CASTING_BLOCKS[0]));
  assert!(game.world.modified.is_empty());assert!(game.given.is_empty());
  if fault!=5{let raw=tconstruct_source_melter_store::read(&game.world,pos,kind).unwrap();assert_eq!(tconstruct_source_melter_state::Casting::from_nbt(&raw).unwrap().tank.amount,0);}
  assert_ne!(&*game.held.as_ref().unwrap().item,"minecraft:bucket");
 }
}
#[test]
fn actual_casting_held_transfer_rejects_aliases_stale_slots_and_failed_save(){
 use horizon_world::nbt::Tag;use std::sync::Arc;
 for mode in 0..4{let(mut game,pos,kind)=portable_casting_game(0,false,1);
  if mode<2{let mut raw=tconstruct_source_melter_store::read(&game.world,pos,kind).unwrap();if let Tag::Compound(fields)=&mut raw{if mode==0{fields.insert("Tank".into(),Tag::Int(1));}else{fields.insert("timer".into(),Tag::String("invalid".into()));}}replace_casting_raw(&mut game,pos,kind,raw);}
  else if mode==2{if let Some(BlockEntity::Chest(slots))=game.world.block_entities.get_mut(&pos){slots[0]=Some(ItemStack::new("minecraft:diamond",1));}}
  else{let chunk=Arc::make_mut(game.world.chunks.get_mut(&(0,0)).unwrap());let saved=Arc::make_mut(chunk.saved.as_mut().unwrap());if let Tag::Compound(fields)=&mut saved.block_entity_sources[0].1{fields.insert("id".into(),Tag::String("other:machine".into()));}}
  let before=game.world.chunks[&(0,0)].saved.as_ref().unwrap().block_entity_sources.clone();let held=game.held.clone();
  interaction::interact_casting(&mut game,pos,CASTING_BLOCKS[0]);
  assert_eq!(game.held,held);assert!(game.given.is_empty());assert!(game.world.modified.is_empty());
  assert_eq!(game.world.chunks[&(0,0)].saved.as_ref().unwrap().block_entity_sources,before);
 }
}
