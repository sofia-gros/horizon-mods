//! Production mask HEAD helper; the native tag lookup remains Horizon's job.
use std::{cell::Cell, ops::ControlFlow};
thread_local! {
    static ANSWER: Cell<Option<bool>> = const { Cell::new(None) };
    static SEEN: Cell<Option<(u128,u32)>> = const { Cell::new(None) };
}
mod sunrise_world { pub(crate) mod level { pub(crate) mod pluto_item_behavior_access {
    pub(crate) fn mask(_: &str, uuid: u128, entity: u32) -> Option<bool> {
        super::super::super::SEEN.set(Some((uuid,entity)));
        super::super::super::ANSWER.get()
    }
}}}
struct Vec3 { x:f64, y:f64, z:f64 }
impl Vec3 { fn new(x:f64,y:f64,z:f64)->Self { Self{x,y,z} } }
struct Mob { id:u32, position:Vec3 }
impl Mob { fn eye_y(&self)->f64 {self.position.y} }
struct Player { head:Option<String>, uuid:[i32;4], alive:bool, look:[f64;3], eye:[f64;3], position:Vec3 }
struct MobWorld<'a> { player:Option<Player>, visible:bool, marker:std::marker::PhantomData<&'a ()> }
impl MobWorld<'_> { fn line_of_sight(&self,_:Vec3,_:Vec3)->bool {self.visible} }
include!("../../../mod/pluto-item-mask-head.rs");

#[test]
fn mask_head_distinguishes_native_fallback_from_false_and_preserves_stare_conditions() {
    let mob=Mob{id:27,position:Vec3::new(0.0,0.0,10.0)};
    let mut world=MobWorld{player:Some(Player{head:Some("example:tagged_disguise".into()),
        uuid:[-1,0,0,7],alive:true,look:[0.0,0.0,1.0],eye:[0.0;3],position:Vec3::new(0.0,0.0,0.0)}),
        visible:true,marker:std::marker::PhantomData};
    ANSWER.set(None);
    assert_eq!(pluto_item_ender_mask(&mob,&world),ControlFlow::Continue(()),"unregistered queries retain native tag lookup");
    assert_eq!(SEEN.get(),Some(((u128::from(u32::MAX)<<96)|7,27)));
    ANSWER.set(Some(true));
    assert_eq!(pluto_item_ender_mask(&mob,&world),ControlFlow::Break(false));
    ANSWER.set(Some(false));
    assert_eq!(pluto_item_ender_mask(&mob,&world),ControlFlow::Break(true),"explicit false overrides disguise, not geometry");
    world.visible=false;
    assert_eq!(pluto_item_ender_mask(&mob,&world),ControlFlow::Break(false));
    world.visible=true;
    world.player.as_mut().unwrap().look=[0.0,0.0,-1.0];
    assert_eq!(pluto_item_ender_mask(&mob,&world),ControlFlow::Break(false));
    world.player.as_mut().unwrap().look=[0.0,0.0,1.0];
    world.player.as_mut().unwrap().alive=false;
    assert_eq!(pluto_item_ender_mask(&mob,&world),ControlFlow::Break(false));
    world.player.as_mut().unwrap().head=None;
    assert_eq!(pluto_item_ender_mask(&mob,&world),ControlFlow::Continue(()));
    world.player=None;
    assert_eq!(pluto_item_ender_mask(&mob,&world),ControlFlow::Continue(()));
    ANSWER.set(None);
}
