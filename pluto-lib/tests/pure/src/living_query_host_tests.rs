//! Executes actual HEAD/LOAD helpers with structural host facades, not native types.
//! It does not execute native fall events, passenger/block dispatch or plan sites.
use std::cell::{Cell,RefCell};
#[path="../../../mod/pluto-living-entity-access.rs"]
pub(crate) mod pluto_living_entity_access;
#[derive(Clone,Copy)]
enum Species {Slime,MagmaCube,SulfurCube,Other}
struct Kind {id:String}
struct Extra {size:i32}
#[derive(Clone,Copy)]
struct Vec3 {x:f64,y:f64,z:f64}
impl Vec3 {fn new(x:f64,y:f64,z:f64)->Self {Self {x,y,z}}}
enum MobEvent {Sound(&'static str,[f64;3],f32,f32)}
struct Mob {id:u32,kind:Kind,extra:Extra,species:Species,attribute:f64,reads:Cell<usize>,
    velocity:Vec3,position:Vec3,fall_distance:f64,on_ground:bool,falls:RefCell<Vec<(f64,f32)>>}
struct MobWorld<'a>(&'a f32);
impl Mob {
    fn species(&self)->Species {self.species}
    fn block_jump_factor(&self,w:&MobWorld<'_>)->f32 {self.reads.set(self.reads.get()+1);*w.0}
    fn mod_block_fall(&self,_contact:Option<(&MobWorld,[i32;3])>,landed:f64,damage:f32)->f32 {
        self.falls.borrow_mut().push((landed,damage));damage
    }
}
mod finalize {
    use super::Mob;
    pub enum Attr {AttackDamage}
    pub fn value(m:&Mob,_:Attr)->f64 {m.reads.set(m.reads.get()+1);m.attribute}
}
mod living {
    use super::{Mob,MobEvent,MobWorld,Vec3};
    include!("../../../mod/pluto-living-entity-jump.rs");
    include!("../../../mod/pluto-living-entity-fall.rs");
    include!("../../../mod/pluto-living-cause-fall.rs");
    pub(super) fn fall(m:&Mob,value:f64,distance:f64,multiplier:f32)->f64 {pluto_living_entity_fall(value,m,distance,multiplier)}
    pub(super) fn query(m:&Mob,w:&MobWorld<'_>)->std::ops::ControlFlow<f32> {pluto_living_entity_jump(m,w)}
    pub(super) fn cause_fall(m:&mut Mob,distance:f64,multiplier:f32,events:&mut Vec<MobEvent>,contact:Option<(&MobWorld,[i32;3])>)->std::ops::ControlFlow<bool> {pluto_living_cause_fall(m,distance,multiplier,events,contact)}
}
mod cube {
    use super::Mob;
    include!("../../../mod/pluto-living-entity-attack.rs");
    pub(super) fn query(m:&Mob)->std::ops::ControlFlow<f32> {pluto_living_entity_attack(m)}
}
#[test]
fn actual_query_helpers_keep_unregistered_fallback_zero_and_native_inputs_without_id_storage() {
    use std::ops::ControlFlow::{Break,Continue};
    let mut mob=Mob {id:17,kind:Kind {id:"query_host:claimed".into()},extra:Extra {size:9},species:Species::Slime,
        attribute:3.000000000000001,reads:Cell::new(0),
        velocity:Vec3::new(0.0,0.0,0.0),position:Vec3::new(1.0,64.0,2.0),fall_distance:0.0,on_ground:true,falls:RefCell::new(Vec::new())};
    let world=MobWorld(&0.5);
    assert_eq!(living::query(&mob,&world),Continue(()));
    assert_eq!(living::fall(&mob,4.0,10.0,0.75),4.0);
    pluto_living_entity_access::set_sink(pluto_living_entity_access::Sink {
        attributes:|_|Ok(Vec::new()),
        has_fall:|kind|kind=="query_host:claimed",
        fall:|id,kind,distance,multiplier,inherited| {
            assert_eq!((id,kind),(17,"query_host:claimed"));
            assert_eq!((distance,multiplier),(10.0,0.75));
            (inherited==4.0).then_some(0)
        },
        has_jump:|kind|kind=="query_host:claimed",has_attack:|kind|kind=="query_host:claimed",
        jump:|id,kind,size,factor| {assert_eq!(id,17);assert_eq!(kind,"query_host:claimed");assert_eq!(factor,0.5);
            size.map(|size|{assert_eq!(size,9);0.0})},
        attack:|id,kind,attribute|{assert_eq!(id,17);assert_eq!(kind,"query_host:claimed");
            assert_eq!(attribute,3.000000000000001);Some(5.0)},
        has_cause_fall:|kind|kind=="query_host:claimed",
        cause_fall:|id,kind,distance,_multiplier,velocity,_suppressing| {
            assert_eq!((id,kind),(17,"query_host:claimed"));
            (distance>2.0).then_some(pluto_living_entity_access::FallOutcome::Bounce {
                velocity:[velocity[0]/0.95,-velocity[1]*0.9,velocity[2]/0.95],sound:Some("query_host:bounce")})
        },
        has_drop_chance:|kind|kind=="query_host:claimed",
        drop_chance:|kind,slot,size,chance| {
            assert_eq!(kind,"query_host:claimed");
            (slot=="head"&&size.is_some_and(|s|s>1)&&chance>0.25).then_some(0.25)
        },
    }).unwrap();
    for species in [Species::Slime,Species::MagmaCube,Species::SulfurCube] {
        mob.species=species;assert_eq!(living::query(&mob,&world),Break(0.0));
    }
    assert_eq!(cube::query(&mob),Break(5.0));
    assert_eq!(living::fall(&mob,4.0,10.0,0.75),0.0);
    assert_eq!(living::fall(&mob,5.0,10.0,0.75),5.0);
    mob.species=Species::Other;assert_eq!(living::query(&mob,&world),Continue(()));
    let before=mob.reads.get();mob.kind.id="minecraft:slime".into();
    assert_eq!(living::query(&mob,&world),Continue(()));assert_eq!(cube::query(&mob),Continue(()));
    assert_eq!(living::fall(&mob,4.0,10.0,0.75),4.0);
    assert_eq!(mob.reads.get(),before,"unregistered native kinds must not evaluate an override's inputs");
    // The cause-fall helper: a decided bounce rewrites the motion, clears the
    // landing state, plays the sound and runs the block's fallOn with zero
    // damage; an unregistered kind continues into the inherited body.
    mob.kind.id="query_host:claimed".into();
    mob.velocity=Vec3::new(0.95,-1.0,0.0);mob.fall_distance=10.0;mob.on_ground=true;
    let mut events:Vec<MobEvent>=Vec::new();
    assert_eq!(living::cause_fall(&mut mob,10.0,1.0,&mut events,None),Break(false));
    assert_eq!((mob.velocity.x,mob.velocity.y,mob.velocity.z),(1.0,0.9,0.0));
    assert_eq!(mob.fall_distance,0.0);assert!(!mob.on_ground);
    assert_eq!(events.len(),1);
    assert_eq!(mob.falls.borrow().as_slice(),&[(10.0,0.0)]);
    mob.kind.id="minecraft:slime".into();
    assert_eq!(living::cause_fall(&mut mob,10.0,1.0,&mut events,None),Continue(()));
}
