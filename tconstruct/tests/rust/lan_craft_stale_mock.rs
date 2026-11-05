#![allow(dead_code,unused_imports)]
use std::collections::HashMap;
mod game {
 use super::*;
 #[derive(Clone,PartialEq,Eq,Debug)] pub struct ItemStack(pub i32);
 #[derive(Clone,Copy,PartialEq,Eq)] pub enum Kind { Crafting, Other }
 pub enum Input { Pickup }
 pub struct Env { pub result: Option<ItemStack> }
 pub struct Menu { pub slots: Vec<Option<ItemStack>>, pub drag: bool, pub kind: Kind }
 impl Menu {
  pub fn slot(&self, n: usize)->Option<&ItemStack>{ self.slots[n].as_ref() }
  pub fn kind(&self)->Kind{self.kind}
  pub fn set_slot(&mut self,e:&Env,n:usize,v:Option<ItemStack>){self.slots[n]=v;if n==1 {self.slots[0]=e.result.clone();}}
  pub fn quick_crafting(&self)->bool{self.drag}
  pub fn click(&mut self,_:&Env,_:i32,_:i32,_:Input)->Result<(),()>{assert!(self.drag);self.drag=false;Ok(())}
 }
 #[derive(Default)] pub struct ChunkMap { pub generation:u64, pub seen:HashMap<(u32,u64,i32),u64> }
 impl ChunkMap {
  pub fn __horizon_mixins_tconstruct_lan_craft_recipe_generation(&self)->&u64{&self.generation}
  pub fn __horizon_mixins_tconstruct_lan_craft_menu_generation(&self)->&HashMap<(u32,u64,i32),u64>{&self.seen}
  pub fn __horizon_mixins_tconstruct_lan_craft_menu_generation_mut(&mut self)->&mut HashMap<(u32,u64,i32),u64>{&mut self.seen}
 }
 pub struct World {pub chunks:ChunkMap} impl World{pub fn block(&self,_:i32,_:i32,_:i32)->i32{0}}
 pub struct Fact{pub name:Box<str>} pub struct Facts{pub fact:Fact} impl Facts{pub fn get(&self,_:i32)->&Fact{&self.fact}}
 pub struct Res{pub facts:Facts} impl Res{pub fn facts(&self)->&Facts{&self.facts}}
 pub struct Game<'a>{pub world:World,pub res:Res,pub result:Option<ItemStack>,pub fallback:Option<ItemStack>,pub prepared:usize,pub sent:usize,pub _phantom:std::marker::PhantomData<&'a ()>}
 impl Game<'_>{pub fn lan_table_valid(&self,_:&Guest,_:[i32;3])->bool{true} pub fn menu_env(&self)->Env{Env{result:self.result.clone()}} pub fn native_prepare_item_craft(&mut self,_:u32,_:&str,_:Option<[i32;3]>,result:&mut Option<ItemStack>){self.prepared+=1;if result.is_none(){*result=self.fallback.clone()}} pub fn lan_send_table_content(&mut self,_:&mut LanHost,_:u32){self.sent+=1}}
 pub struct Guest{pub conn:u32,pub placed_at:u64,pub uuid:u32,pub name:String,pub spectator:bool} impl Guest{pub fn spectator(&self)->bool{self.spectator}}
 pub struct Hub{pub guests:Vec<Guest>} pub struct Crafting{pub table:Option<(i32,i32,Box<Menu>,[i32;3])>} pub struct Play{pub crafting:Crafting} impl Default for Play {fn default()->Self{Self{crafting:Crafting{table:None}}}}
 pub struct LanHost{pub hub:Hub,pub play:HashMap<u32,Play>}
 pub mod play {pub struct ContainerClick{pub container:i32}}
 pub mod lan_crafting { use super::*; pub mod tconstruct_lan_craft_stale_guard { include!(concat!(env!("PWD"), "/mixins/tconstruct/source-lan-craft-stale.rs")); } }
}
use game::*; use std::ops::ControlFlow;
fn setup(old:i32,new:Option<i32>)->(Game<'static>,LanHost,play::ContainerClick){
 let g=Game{world:World{chunks:ChunkMap{generation:1,..Default::default()}},res:Res{facts:Facts{fact:Fact{name:"minecraft:crafting_table".into()}}},result:new.map(ItemStack),fallback:None,prepared:0,sent:0,_phantom:std::marker::PhantomData};
 let mut slots=vec![None;46];slots[0]=Some(ItemStack(old));slots[1]=Some(ItemStack(7));
 let m=Menu{slots,drag:false,kind:Kind::Crafting};let p=Play{crafting:Crafting{table:Some((3,4,Box::new(m),[0,0,0]))}};
 let h=LanHost{hub:Hub{guests:vec![Guest{conn:1,placed_at:9,uuid:10,name:"test".into(),spectator:false}]},play:HashMap::from([(1,p)])};(g,h,play::ContainerClick{container:3})
}
#[test] fn changed_result_cancels_and_sends_once(){let(mut g,mut h,p)=setup(1,None);assert!(matches!(game::lan_crafting::tconstruct_lan_craft_stale_guard::before_click(&mut g,&mut h,1,&p),ControlFlow::Break(true)));assert_eq!(g.sent,1);assert_eq!(g.prepared,1);assert!(h.play[&1].crafting.table.as_ref().unwrap().2.slot(0).is_none());assert!(matches!(game::lan_crafting::tconstruct_lan_craft_stale_guard::before_click(&mut g,&mut h,1,&p),ControlFlow::Continue(())));assert_eq!(g.prepared,1);}
#[test] fn unchanged_continues_and_fallback_restored(){let(mut g,mut h,p)=setup(1,None);g.fallback=Some(ItemStack(1));assert!(matches!(game::lan_crafting::tconstruct_lan_craft_stale_guard::before_click(&mut g,&mut h,1,&p),ControlFlow::Continue(())));assert_eq!(g.sent,0);assert_eq!(g.prepared,1);}
#[test] fn drag_reset_and_changed_count(){let(mut g,mut h,p)=setup(1,Some(2));h.play.get_mut(&1).unwrap().crafting.table.as_mut().unwrap().2.drag=true;assert!(matches!(game::lan_crafting::tconstruct_lan_craft_stale_guard::before_click(&mut g,&mut h,1,&p),ControlFlow::Break(true)));assert!(!h.play[&1].crafting.table.as_ref().unwrap().2.drag);}
#[test] fn wrong_window_and_other_block_untouched(){let(mut g,mut h,mut p)=setup(1,None);p.container=4;assert!(matches!(game::lan_crafting::tconstruct_lan_craft_stale_guard::before_click(&mut g,&mut h,1,&p),ControlFlow::Continue(())));p.container=3;g.res.facts.fact.name="tconstruct:crafting_station".into();assert!(matches!(game::lan_crafting::tconstruct_lan_craft_stale_guard::before_click(&mut g,&mut h,1,&p),ControlFlow::Continue(())));assert_eq!(g.prepared,0);}
