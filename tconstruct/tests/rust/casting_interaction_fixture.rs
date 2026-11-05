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
struct Facts;struct Fact {name:String}impl Facts{fn get(&self,_:u8)->Fact{Fact{name:"tconstruct:seared_table".into()}}}
struct Resources;impl Resources{fn items(&self)->HashMap<String,()>{HashMap::new()}fn facts(&self)->Facts{Facts}}
struct Stats;impl Stats{fn add(&mut self,_:&str,_:&str,_:u32){}}
struct Game<'a>{world:tconstruct_world::World,held:Option<ItemStack>,given:Vec<ItemStack>,creative:bool,lan:Option<()>,lan_acting_uuid:Option<()>,res:Resources,stats:Stats,marker:std::marker::PhantomData<&'a ()>}
impl Game<'_>{
 fn held(&self)->Option<&ItemStack>{self.held.as_ref()}fn held_mut(&mut self)->&mut Option<ItemStack>{&mut self.held}
 fn ensure_datapacks(&mut self){}fn give_or_drop(&mut self,s:ItemStack){self.given.push(s)}fn native_acting_id(&self)->u8{0}
 fn native_item_crafted(&mut self,_:u8,_:&str,_:&ItemStack){}
}
impl tconstruct_world::World{fn block(&self,_:i32,_:i32,_:i32)->u8{0}}
mod tconstruct_source_fluid_container {
 use super::*;pub enum Direction{EmptyItem,Auto}
 pub fn casting_portable_rewrite_safe(_: &ItemStack)->bool{true}
 pub fn transfer_single_with_color(_: &mut tconstruct_source_melter_state::Tank,_:&mut [Option<ItemStack>],_:Direction,_:&impl Fn(&str)->bool,_:&impl Fn(&str)->bool,_:Option<&HashMap<String,HashSet<String>>>,_:bool)->bool{false}
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
 fn kind(_: &str)->Option<&'static str>{Some("tconstruct:table")}
 struct Fluid{amount:u32}struct Recipe{fluid:Fluid,cooling_time:u32,consumes_cast:bool,switch_slots:bool}
 fn casting_recipe_at(_: &Game<'_>,_:Pos,_:&str,_:&Casting,_:&Tank)->Option<(String,Recipe,ItemStack)>{None}
 // __CASTING_AT__
 // __INTERACT_CASTING__
}
fn interaction_game(input:Option<ItemStack>,held:Option<ItemStack>)->(Game<'static>,Pos){
 let mut state=tconstruct_source_melter_state::Casting::new();state.input=input.clone();
 let(world,pos)=casting_world(state.to_nbt());let mut game=Game{world,held,given:vec![],creative:false,lan:None,lan_acting_uuid:None,res:Resources,stats:Stats,marker:std::marker::PhantomData};
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
