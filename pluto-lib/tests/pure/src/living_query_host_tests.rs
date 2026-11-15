//! Executes actual HEAD/LOAD helpers with structural host facades, not native types.
//! It does not execute native fall events, passenger/block dispatch or plan sites.
use std::cell::Cell;
#[path="../../../mod/pluto-living-entity-access.rs"]
pub(crate) mod pluto_living_entity_access;
#[derive(Clone,Copy)]
enum Species {Slime,MagmaCube,SulfurCube,Other}
struct Kind {id:String}
struct Extra {size:i32}
struct Mob {id:u32,kind:Kind,extra:Extra,species:Species,attribute:f64,reads:Cell<usize>}
struct MobWorld<'a>(&'a f32);
impl Mob {
    fn species(&self)->Species {self.species}
    fn block_jump_factor(&self,w:&MobWorld<'_>)->f32 {self.reads.set(self.reads.get()+1);*w.0}
}
mod finalize {
    use super::Mob;
    pub enum Attr {AttackDamage}
    pub fn value(m:&Mob,_:Attr)->f64 {m.reads.set(m.reads.get()+1);m.attribute}
}
mod living {
    use super::{Mob,MobWorld};
    include!("../../../mod/pluto-living-entity-jump.rs");
    include!("../../../mod/pluto-living-entity-fall.rs");
    pub(super) fn fall(m:&Mob,value:f64,distance:f64,multiplier:f32)->f64 {pluto_living_entity_fall(value,m,distance,multiplier)}
    pub(super) fn query(m:&Mob,w:&MobWorld<'_>)->std::ops::ControlFlow<f32> {pluto_living_entity_jump(m,w)}
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
        attribute:3.000000000000001,reads:Cell::new(0)};
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
}
