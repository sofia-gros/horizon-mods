//! Actual particle injection helper with structural Game/registry fixtures.
//! Native particle construction, biome blending, attachment and rendering are not tested.
use std::{collections::BTreeMap,marker::PhantomData,sync::{Arc,Mutex}};
use crate::{BlockColorContext,ColorProviderRegistry,Identifier};
#[derive(Clone,Copy)]
struct BlockStateId(u32);
struct State {name:String,properties:BTreeMap<String,String>}
struct Registry(BTreeMap<u32,State>);
impl Registry {fn state(&self,id:BlockStateId)->Option<&State>{self.0.get(&id.0)}}
struct Assets {registry:Registry}
struct Pack {assets:Assets}
struct Game<'a> {pack:Pack,lifetime:PhantomData<&'a ()>}
mod sunrise_client {
    pub mod mesher {
        pub mod pluto_block_colors {
            pub(crate) fn color_in(registry:&super::super::super::Registry,id:super::super::super::BlockStateId,pos:Option<[i32;3]>,index:i32)->Option<u32> {
                let state=registry.state(id)?;
                crate::pure::color_provider_api_v1::block_color_from_host(&state.name,&state.properties,pos,index).map(|rgb|rgb&0xFF_FFFF)
            }
        }
    }
}
include!("../../../mod/pluto-block-particle-tint.rs");

#[test]
fn actual_particle_helper_uses_supplied_registry_position_state_and_tint_zero_without_replacing_native_fallback() {
    let seen=Arc::new(Mutex::new(Vec::<BlockColorContext>::new())); let captured=seen.clone();
    ColorProviderRegistry::register_block(&[Identifier::parse("example:particle_leaves").unwrap()],move |context| {
        captured.lock().unwrap().push(context.clone());
        (context.state.property("persistent")==Some("true")).then_some(0xAB123456)
    });
    ColorProviderRegistry::register_block(&[Identifier::parse("minecraft:grass_block").unwrap()],|_|panic!("ordinary grass dirt crumbs must not query tint"));
    let game=|name:&str,persistent:&str|Game {pack:Pack {assets:Assets {registry:Registry([
        (7,State{name:name.into(),properties:[("persistent".into(),persistent.into())].into_iter().collect()}),
        (8,State{name:"minecraft:grass_block".into(),properties:Default::default()}),
    ].into_iter().collect())}},lifetime:PhantomData};
    let first=game("example:particle_leaves","true"); let second=game("example:particle_leaves","false");
    let pos=[-17,71,1025]; let native=0x556677;
    assert_eq!(pluto_block_particle_tint(native,&first,BlockStateId(7),pos),0x123456);
    assert_eq!(pluto_block_particle_tint(native,&second,BlockStateId(7),pos),native,"same numeric ID in a different registry must not reuse earlier properties");
    assert_eq!(pluto_block_particle_tint(native,&first,BlockStateId(8),pos),native,"vanilla grass exception precedes provider dispatch");
    assert_eq!(pluto_block_particle_tint(native,&first,BlockStateId(999),pos),native,"missing registry state keeps native result");
    let other=game("example:unregistered_particle_block","true");
    assert_eq!(pluto_block_particle_tint(native,&other,BlockStateId(7),pos),native);
    let seen=seen.lock().unwrap(); assert_eq!(seen.len(),2);
    for context in seen.iter() {assert_eq!(context.pos,Some(pos)); assert_eq!(context.tint_index,0);}
    assert_eq!(seen[0].state.property("persistent"),Some("true"));
    assert_eq!(seen[1].state.property("persistent"),Some("false"));
}
