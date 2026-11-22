#![allow(dead_code)]
// Host doubles expose only calls the production adapter needs. Native hurt is
// deliberately observable and moves both positions, like an immediate teleport.
extern crate self as horizon_world;
extern crate self as horizon_core;
use std::sync::Arc;
#[derive(Clone, Copy, Debug)]
struct Vec3 { x: f64, y: f64, z: f64 }
impl Vec3 {
    fn new(x:f64,y:f64,z:f64)->Self { Self{x,y,z} }
    fn subtract(self,b:Self)->Self { Self::new(self.x-b.x,self.y-b.y,self.z-b.z) }
    fn add_vector(self,b:Self)->Self { Self::new(self.x+b.x,self.y+b.y,self.z+b.z) }
    fn scale(self,s:f64)->Self { Self::new(self.x*s,self.y*s,self.z*s) }
    fn horizontal_distance_squared(self)->f64 { self.x*self.x+self.z*self.z }
}
struct Bounds(Vec3);
impl Bounds {
    fn deflate(self,_:Vec3)->Self { self }
    fn min_position(&self)->Vec3 { self.0 }
    fn max_position(&self)->Vec3 { self.0 }
}
#[derive(Clone,Copy,PartialEq)]
enum Species { Turtle, Bat, Spider, CaveSpider, Wither, Fox, Bee, Enderman }
#[derive(Debug,PartialEq)]
enum DamageKind { HotFloor, Cactus, Campfire, SweetBerryBush }
struct Damage(DamageKind);
impl Damage { fn of(k:DamageKind)->Self { Self(k) } }
struct Kind { id:Arc<str> }
struct Mob {
    id:u32, kind:Kind, position:Vec3, previous:Vec3, velocity:Vec3,
    on_ground:bool, needs_sync:bool, fall_distance:f64, stuck_multiplier:Vec3,
    direct_damage:usize,
}
impl Mob {
    fn alive(&self)->bool { true }
    fn block_on_legacy(&self,_:&MobWorld)->[i32;3] { [0,-1,0] }
    fn species(&self)->Species { Species::Enderman }
    fn bounds_at(&self,p:Vec3)->Bounds { Bounds(p) }
    fn is_on_fire(&self)->bool { false }
    fn extinguish_fire(&mut self) {}
    fn has_effect(&self,_:&str)->bool { false }
    fn damage(&mut self,_:f32,_:Damage) { self.direct_damage+=1; }
}
#[derive(Debug)]
enum MobEvent {
    NativeHurt(DamageKind), CrackTurtleEgg {pos:[i32;3]},
    BreakBlock {pos:[i32;3],drop:bool}, PowderSnowParticle {at:[f64;3],velocity:[f64;3]},
}
struct LegacyRandom;
impl LegacyRandom { fn next_int_bound(&mut self,_:i32)->Option<i32> { Some(1) } }
struct World { attachments:() }
impl World { fn block(&self,_:i32,_:i32,_:i32)->u32 { 0 } }
struct Definition { name:String, properties:Vec<(String,String)> }
struct Registry;
impl Registry { fn state(&self,_:u32)->Option<Definition> { None } }
struct MobWorld<'a> {
    world:&'a World, registry:Registry, mob_griefing:bool, game_time:i64,
    controlling_player_movement:Option<Vec3>,
}
impl MobWorld<'_> {
    fn block_name(&self,x:i32,y:i32,_:i32)->&'static str {
        match (x,y) { (0,-1)=>"minecraft:magma_block",(0,0)=>"minecraft:cactus",
            (1,0)=>"minecraft:campfire",_=>"minecraft:air" }
    }
    fn prop(&self,_:i32,_:i32,_:i32,_:&str)->Option<&str> { Some("true") }
}
fn hurt(m:&mut Mob,_:&MobWorld,events:&mut Vec<MobEvent>,_:f32,damage:Damage) {
    events.push(MobEvent::NativeHurt(damage.0));
    m.position=Vec3::new(100.0,0.0,0.0);
    m.previous=m.position;
}
fn powder_snow_particle_velocity(_:Vec3,_:&mut LegacyRandom)->Option<[f64;3]> { None }
fn honey_slide(_:&mut Mob,_:[i32;3],_:&mut LegacyRandom,_:&mut Vec<MobEvent>) {}
pub mod position { pub struct BlockPos; impl BlockPos { pub fn new(_:i32,_:i32,_:i32)->Self { Self } } }
mod block_contact {
    pub struct Contact<'a> { pub entity:i32,pub kind:&'a str,pub block:&'a str,pub state:u32,pub position:crate::position::BlockPos }
    pub enum Event<'a> { StepOn(Contact<'a>),Inside(Contact<'a>) }
    pub fn emit(_:Event) {}
    pub fn current_level()->Option<&'static str> { None }
}
pub mod level {
    pub mod pluto_block_access {
        #[derive(Clone)]
        pub struct StateInfo { pub block:String,pub properties:Vec<(String,String)> }
        pub struct EntityInside { pub dimension:String,pub state:StateInfo,pub pos:[i32;3],pub entity:i32,
            pub kind:std::sync::Arc<str>,pub bounds:[f64;6],pub fall_distance:f64 }
        pub struct InsideMotion {
            pub pos:[i32;3],pub entity:i32,pub kind:std::sync::Arc<str>,pub server:bool,
            pub suppressing_bounce:bool,pub position:[f64;3],pub previous_position:[f64;3],
            pub velocity:[f64;3],pub sync_velocity:bool,pub step_sound:Option<[f32;2]>,
        }
        pub struct Sink { pub inside_motion:fn(&StateInfo)->Option<fn(&mut InsideMotion)>,
            pub entity_inside:fn(&EntityInside) }
        pub fn landing_sink()->Option<Sink> { None }
    }
    pub mod pluto_contact_effects {
        pub struct ContactSound { pub state:u32,pub pos:[i32;3],pub volume:f32,pub pitch:f32 }
        pub fn world_active(_:&(),_:i64)->bool { false }
        pub fn queue_world(_:&(),_:i32,_:i64,_:ContactSound)->Result<(),()> { Ok(()) }
    }
}
mod adapter { include!(PRODUCTION_HELPER); }
#[test]
fn damage_teleports_immediately_without_retargeting_the_sweep() {
    let world=World{attachments:()};
    let context=MobWorld{world:&world,registry:Registry,mob_griefing:true,game_time:1,controlling_player_movement:None};
    let mut mob=Mob{id:1,kind:Kind{id:"minecraft:enderman".into()},position:Vec3::new(1.0,0.0,0.0),
        previous:Vec3::new(0.0,0.0,0.0),velocity:Vec3::new(0.0,0.0,0.0),on_ground:true,
        needs_sync:false,fall_distance:0.0,stuck_multiplier:Vec3::new(1.0,1.0,1.0),direct_damage:0};
    let mut events=Vec::new();
    adapter::effects(&mut mob,&context,&mut LegacyRandom,&mut events);
    assert_eq!(mob.direct_damage,0,"adapter must delegate each contact hurt to native teleport-aware hurt");
    let hurts:Vec<_>=events.iter().filter_map(|e| if let MobEvent::NativeHurt(k)=e {Some(k)} else {None}).collect();
    assert_eq!(hurts,vec![&DamageKind::HotFloor,&DamageKind::Cactus,&DamageKind::Campfire],
        "stepOn and each swept cell hurt once along the original path, even after teleport");
    assert_eq!(mob.position.x,100.0);
}
