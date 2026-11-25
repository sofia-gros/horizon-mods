//! Actual factory bridge and HEAD helper, with a trace Game facade. Native
//! model baking, stack conversion, filters and particle physics are unverified.
use std::{marker::PhantomData,ops::ControlFlow};
use crate::{Identifier,ParticleFactoryRegistry,StackData,Nbt};
#[derive(Clone,Debug,PartialEq)]
pub(crate) struct ParticleOptions {mod_particle:Option<String>,item:Option<StackData>,model_owner:u32}
impl ParticleOptions {fn named(name:impl Into<String>)->Self {Self {mod_particle:Some(name.into()),item:None,model_owner:0}}}
#[derive(Clone,Debug,PartialEq)]
pub(crate) enum MobEvent {Particles {kind:&'static str,at:[f64;3],count:u32},Other}
struct MobType {id:String}
struct Mob {kind:MobType}
pub(crate) mod pluto_entity_particles {include!("../../../mod/pluto-entity-particles.rs");}
mod cube {
    use super::Mob;
    include!("../../../mod/pluto-entity-particle-kind.rs");
    pub(super) fn kind(value:&'static str,m:&Mob)->&'static str {pluto_entity_particle_kind(value,m)}
}
#[derive(Debug,PartialEq)]
enum Call {Model(u32,StackData),Native(ParticleOptions,[f64;3],[f64;3])}
struct Game<'a> {owner:u32,accept:bool,calls:Vec<Call>,lifetime:PhantomData<&'a ()>}
impl Game<'_> {
    fn item_particle(&mut self,stack:&StackData)->ParticleOptions {
        self.calls.push(Call::Model(self.owner,stack.clone()));
        ParticleOptions {mod_particle:None,item:Some(stack.clone()),model_owner:self.owner}
    }
    fn native_add_particle(&mut self,options:&ParticleOptions,at:[f64;3],velocity:[f64;3])->bool {
        match pluto_item_particle_factory(self,options,at,velocity) {
            ControlFlow::Break(result)=>result,
            ControlFlow::Continue(())=>{self.calls.push(Call::Native(options.clone(),at,velocity));self.accept},
        }
    }
}
mod horizon_client {
    pub(crate) mod items {pub(crate) type ItemStack=crate::StackData;}
    pub(crate) mod particles {pub(crate) use super::super::ParticleOptions;}
    pub(crate) mod mobs {pub(crate) use super::super::MobEvent;}
}
pub(crate) mod bridge {
    use super::horizon_client;
    include!("../../../mod/pluto-item-particles.rs");
}
include!("../../../mod/pluto-item-particle-head.rs");
include!("../../../mod/pluto-entity-particle-event.rs");

#[test]
fn actual_factory_head_delegates_once_with_own_game_model_stack_position_velocity_and_native_result() {
    bridge::set_provider(crate::pure::particle_factory_registry::item_from_host).unwrap();
    let data=StackData {item:"example:crumb_item".into(),count:2,damage:7,max_damage:Some(30),
        custom_data:[("test:kept".into(),Nbt::Int(3))].into_iter().collect(),custom_model_data:None,can_place_on:None,enchantments:Default::default()};
    ParticleFactoryRegistry::register_item(Identifier::parse("example:crumb_factory").unwrap(),data.clone()).unwrap();
    let at=[-2.0,70.5,19.0];let velocity=[0.25,-0.1,0.75];
    let request=ParticleOptions {mod_particle:Some("example:crumb_factory".into()),item:None,model_owner:0};
    for (owner,accept) in [(1,true),(2,false)] {
        let mut game=Game {owner,accept,calls:vec![],lifetime:PhantomData};
        assert_eq!(game.native_add_particle(&request,at,velocity),accept);
        assert_eq!(game.calls,vec![Call::Model(owner,data.clone()),Call::Native(
            ParticleOptions {mod_particle:None,item:Some(data.clone()),model_owner:owner},at,velocity)],
            "factory conversion delegates exactly once to native validation/add, even on refusal");
    }
    let mut game=Game {owner:3,accept:true,calls:vec![],lifetime:PhantomData};
    let mut unknown=request.clone();unknown.mod_particle=Some("example:unregistered_factory".into());
    assert!(game.native_add_particle(&unknown,at,velocity));
    assert_eq!(game.calls,vec![Call::Native(unknown,at,velocity)],"unknown factories remain native named particles");
    game.calls.clear();let ordinary=ParticleOptions {mod_particle:None,item:None,model_owner:0};
    assert!(game.native_add_particle(&ordinary,at,velocity));
    assert_eq!(game.calls,vec![Call::Native(ordinary,at,velocity)]);
    ParticleFactoryRegistry::register_entity_override(Identifier::parse("example:emit_cube").unwrap(),
        Identifier::parse("minecraft:item_slime").unwrap(),Identifier::parse("example:crumb_factory").unwrap()).unwrap();
    pluto_entity_particles::set_provider(crate::pure::particle_factory_registry::entity_particle_from_host).unwrap();
    let mob=Mob {kind:MobType {id:"example:emit_cube".into()}};
    let kind=cube::kind("item_slime",&mob);
    assert_eq!(kind,"example:crumb_factory");
    assert_eq!(cube::kind("flame",&mob),"flame");
    assert_eq!(cube::kind("item_slime",&Mob {kind:MobType {id:"minecraft:slime".into()}}),"item_slime");
    game.calls.clear();
    let event=MobEvent::Particles {kind,at,count:0};
    assert_eq!(pluto_entity_particle_event(event.clone(),&mut game),event,"the event is forwarded unchanged for native consumers");
    assert_eq!(game.calls,vec![Call::Model(3,data.clone()),Call::Native(
        ParticleOptions {mod_particle:None,item:Some(data),model_owner:3},at,[0.0;3])]);
    game.calls.clear();
    for event in [MobEvent::Other,MobEvent::Particles {kind,at,count:2},
        MobEvent::Particles {kind:"item_slime",at,count:0},
        MobEvent::Particles {kind:"minecraft:item_slime",at,count:0},
        MobEvent::Particles {kind:"example:missing_factory",at,count:0}] {
        assert_eq!(pluto_entity_particle_event(event.clone(),&mut game),event);
        assert!(game.calls.is_empty(),"unhandled/count-burst events must not create a partial or duplicate particle");
    }
}
