//! Actual opt-in HEAD/provider helpers with a trace Game facade; this does not
//! execute the adapted native Game death pipeline or validate Horizon types.
use std::{marker::PhantomData,ops::ControlFlow,sync::{Arc,atomic::{AtomicUsize,Ordering}}};
mod horizon_client {pub(crate) mod items {pub(crate) struct ItemStack;}}
mod horizon_world {pub(crate) mod loot {
    pub(crate) struct LootEntity {pub kind:String,pub clones:std::sync::Arc<std::sync::atomic::AtomicUsize>}
    impl Clone for LootEntity {fn clone(&self)->Self {self.clones.fetch_add(1,std::sync::atomic::Ordering::SeqCst);Self {kind:self.kind.clone(),clones:self.clones.clone()}}}
}}
struct Mob {id:u32}
struct Game<'a> {calls:Vec<(u32,Option<u128>,bool,String,bool,[f64;3])>,lifetime:PhantomData<&'a ()>}
impl Game<'_> {
    fn pluto_drop_death_loot(&mut self,m:&mut Mob,player:Option<u128>,by_player:bool,killer:horizon_world::loot::LootEntity,consumed:bool,pos:[f64;3]) {
        self.calls.push((m.id,player,by_player,killer.kind,consumed,pos));
    }
}
pub(crate) mod bridge {
    use super::horizon_client;
    include!("../../../mod/pluto-living-drops.rs");
}
include!("../../../mod/pluto-living-drops-head.rs");
#[test]
fn actual_death_head_preserves_no_provider_and_no_callbacks_fallback_without_moving_killer() {
    // ExplosionSource carries MobType's raw ID, whereas loot predicates need
    // canonical resource locations. Preserve custom killers without breaking
    // vanilla's charged-head table when its retained ID is simply "creeper".
    for (raw,expected) in [("creeper","minecraft:creeper"),("minecraft:creeper","minecraft:creeper"),
        ("example:derived_creeper","example:derived_creeper")] {
        assert_eq!(bridge::entity_kind(raw),expected);
    }
    static ACTIVE:AtomicUsize=AtomicUsize::new(0);
    let clones=Arc::new(AtomicUsize::new(0));
    let killer=horizon_world::loot::LootEntity {kind:"minecraft:creeper".into(),clones:clones.clone()};
    let mut game=Game {calls:vec![],lifetime:PhantomData};let mut mob=Mob {id:17};let pos=[-1.0,70.0,12.0];
    assert_eq!(pluto_living_drops_head(&mut game,&mut mob,Some(3),true,&killer,false,pos),ControlFlow::Continue(()));
    bridge::set_provider(||match ACTIVE.load(Ordering::SeqCst) {0=>Ok(false),1=>Ok(true),_=>Err("registry error".into())},|_|Ok(())).unwrap();
    assert_eq!(pluto_living_drops_head(&mut game,&mut mob,Some(3),true,&killer,false,pos),ControlFlow::Continue(()));
    assert!(game.calls.is_empty());assert_eq!(clones.load(Ordering::SeqCst),0);
    assert_eq!(killer.kind,"minecraft:creeper","native fallback still owns its original damage-source argument");
    ACTIVE.store(1,Ordering::SeqCst);
    assert_eq!(pluto_living_drops_head(&mut game,&mut mob,Some(3),true,&killer,false,pos),ControlFlow::Break(()));
    assert_eq!(game.calls,vec![(17,Some(3),true,"minecraft:creeper".into(),false,pos)]);
    assert_eq!(clones.load(Ordering::SeqCst),1);
    ACTIVE.store(2,Ordering::SeqCst);
    assert_eq!(pluto_living_drops_head(&mut game,&mut mob,None,false,&killer,true,pos),ControlFlow::Break(()));
    assert_eq!(game.calls[1],(17,None,false,"minecraft:creeper".into(),true,pos));
    assert_eq!(clones.load(Ordering::SeqCst),2,"registry failure retains the counter-aware path, not the old powered-clearing fallback");
}
