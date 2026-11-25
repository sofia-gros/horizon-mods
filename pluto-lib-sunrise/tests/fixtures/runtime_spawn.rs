// Trace-only host doubles execute the production helper; not native gameplay.
extern crate self as sunrise_client;
extern crate self as log;
#[macro_export] macro_rules! error { ($($arg:tt)*) => { eprintln!($($arg)*) }; }
use std::{collections::HashMap, marker::PhantomData, ops::ControlFlow, sync::atomic::{AtomicBool, Ordering}};
static ENABLED: AtomicBool = AtomicBool::new(true);
#[derive(Clone, Copy)] enum SpawnYaw { Level, Created, Fixed(f32) }
#[derive(Clone, Copy)] struct Vec3;
impl Vec3 { fn new(_:f64,_:f64,_:f64)->Self { Self } }
struct Mob { id:u32, persistent:bool, yaw:f32, own:u32 }
impl Mob { fn with_random(id:u32,_:(),_:Vec3,yaw:f32,own:u32)->Self { Self {id,persistent:false,yaw,own} } }
struct Random { draws:usize }
impl Random { fn next_float(&mut self)->f32 { self.draws+=1; 0.75 } }
struct World { random:Random }
struct Persist { uuids:HashMap<u32,u128> }
struct Resources { types:HashMap<String,()> }
impl Resources { fn mob_types(&self)->&HashMap<String,()> { &self.types } }
struct Game<'a> { next_entity:u32, world:World, persist:Persist, res:Resources, mobs:Vec<Mob>, trace:Vec<&'static str>, marker:PhantomData<&'a ()> }
impl Game<'_> {
 fn biome_facts(&self,_:i32,_:i32,_:i32) {} fn local_difficulty(&self,_:[f64;3]) {}
 fn native_mobs_joined(&mut self,index:usize,_:bool) { assert_eq!(self.persist.uuids.get(&self.mobs[index].id),Some(&123)); self.trace.push("join"); }
 fn enchant_spawn_equipment(&mut self,_:u32) { self.trace.push("gear"); }
 fn halloween_spawn_head(&mut self,_:u32) { self.trace.push("season"); }
 fn spawn_jockeys(&mut self,_:u32,_:bool) { self.trace.push("jockey"); }
}
mod camera { pub fn wrap_degrees(v:f32)->f32 { (v+180.0).rem_euclid(360.0)-180.0 } }
mod mobs {
 pub fn living_random()->(u32,u128) { (77,123) }
 pub mod raider { pub enum Reason { Other,Natural } pub fn finalize(_: &mut crate::Mob,_:Reason) {} }
}
mod spawning { pub fn finalize(_: &mut crate::Mob,_:&(),_:&()) {} }
mod entry { pub mod pluto_entity_spawn_access {
 pub fn enabled()->Result<bool,String> { Ok(crate::ENABLED.load(crate::Ordering::SeqCst)) }
} }
mod game { pub mod pluto_entity_spawn_game {
 pub fn prepare(_: &mut crate::Game<'_>,_: &mut crate::Mob,_:&str)->Result<(),String> { Ok(()) }
 pub fn take(_: &crate::Mob)->Result<Option<()>,String> { Ok(None) }
 pub fn complete(_: &mut crate::Game<'_>,_:u32,_:())->Result<(),String> { Ok(()) }
} }
include!(PRODUCTION_HELPER);
fn fresh()->Game<'static> { Game { next_entity:0,world:World {random:Random {draws:0}},persist:Persist {uuids:HashMap::new()},res:Resources {types:HashMap::from([("minecraft:pig".into(),())])},mobs:Vec::new(),trace:Vec::new(),marker:PhantomData } }
#[test] fn preserves_native_yaw_rng_uuid_and_fallback() {
 for (yaw,expected,draws) in [(SpawnYaw::Level,270.0,1),(SpawnYaw::Created,-90.0,1),(SpawnYaw::Fixed(42.0),42.0,0)] {
  let mut game=fresh();
  assert_eq!(pluto_entity_spawn_runtime(&mut game,"pig",[0.0;3],false,false,true,true,yaw),ControlFlow::Break(Some(1)));
  assert_eq!(game.mobs[0].yaw,expected); assert_eq!(game.mobs[0].own,77);
  assert_eq!(game.world.random.draws,draws);
  assert_eq!(game.trace,["join","gear","season","jockey"]);
 }
 let mut game=fresh();
 assert_eq!(pluto_entity_spawn_runtime(&mut game,"pig",[0.0;3],false,false,false,false,SpawnYaw::Created),ControlFlow::Continue(()));
 assert_eq!(game.next_entity,0); assert_eq!(game.world.random.draws,0); assert!(game.persist.uuids.is_empty());
 ENABLED.store(false,Ordering::SeqCst);
 assert_eq!(pluto_entity_spawn_runtime(&mut game,"pig",[0.0;3],false,false,false,true,SpawnYaw::Created),ControlFlow::Continue(()));
 assert_eq!(game.next_entity,0); assert!(game.mobs.is_empty());
 ENABLED.store(true,Ordering::SeqCst);
}
