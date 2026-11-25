//! Actual opt-in HEAD/provider helpers with a trace Game facade; this does not
//! execute the adapted native Game death pipeline or validate Horizon types.
use std::{marker::PhantomData,ops::ControlFlow,sync::{Arc,atomic::{AtomicUsize,Ordering}}};
mod sunrise_client {pub(crate) mod items {pub(crate) struct ItemStack;}}
mod sunrise_world {pub(crate) mod loot {
    pub(crate) struct LootStack {pub item:String,pub count:i32,pub enchantments:Vec<(String,i32)>}
    impl LootStack {pub(crate) fn is_empty(&self)->bool {self.count<=0||self.item=="minecraft:air"}}
    pub(crate) struct LootEntity {pub kind:String,pub clones:std::sync::Arc<std::sync::atomic::AtomicUsize>,pub equipment:Vec<(String,LootStack)>}
    impl LootEntity {pub(crate) fn item(&self,slot:&str)->Option<&LootStack> {self.equipment.iter().find(|(s,i)|s==slot&&!i.is_empty()).map(|(_,i)|i)}}
    impl Clone for LootEntity {fn clone(&self)->Self {self.clones.fetch_add(1,std::sync::atomic::Ordering::SeqCst);Self {kind:self.kind.clone(),clones:self.clones.clone(),equipment:self.equipment.iter().map(|(s,i)|(s.clone(),LootStack {item:i.item.clone(),count:i.count,enchantments:i.enchantments.clone()})).collect()}}}
}}
struct Mob {id:u32}
struct Game<'a> {calls:Vec<(u32,Option<u128>,bool,String,bool,[f64;3])>,lifetime:PhantomData<&'a ()>}
impl Game<'_> {
    fn pluto_drop_death_loot(&mut self,m:&mut Mob,player:Option<u128>,by_player:bool,killer:sunrise_world::loot::LootEntity,consumed:bool,pos:[f64;3]) {
        self.calls.push((m.id,player,by_player,killer.kind,consumed,pos));
    }
}
pub(crate) mod bridge {
    use super::{sunrise_client, sunrise_world};
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
    let killer=sunrise_world::loot::LootEntity {kind:"minecraft:creeper".into(),clones:clones.clone(),equipment:vec![]};
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
#[test]
fn killer_mainhand_looting_reads_the_killer_hand_not_the_other_slots() {
    // A LAN guest's kill: the killer LootEntity carries the guest's own hand
    // (the host resolved it through as_guest). Only the main hand counts: a
    // Looting offhand or helm must not raise the level.
    let stack=|item:&str,enchants:Vec<(String,i32)>| sunrise_world::loot::LootStack {item:item.into(),count:1,enchantments:enchants};
    let killer=|main:Vec<(String,i32)>| sunrise_world::loot::LootEntity {kind:"minecraft:player".into(),clones:Arc::new(AtomicUsize::new(0)),equipment:vec![
        ("mainhand".into(),stack("minecraft:diamond_sword",main)),
        ("offhand".into(),stack("minecraft:shield",vec![("minecraft:looting".into(),5)])),
        ("head".into(),stack("minecraft:diamond_helmet",vec![("minecraft:looting".into(),2)]))]};
    assert_eq!(bridge::killer_mainhand_looting(&killer(vec![("minecraft:looting".into(),3)])),3);
    assert_eq!(bridge::killer_mainhand_looting(&killer(vec![("minecraft:sharpness".into(),2)])),0);
    let empty=sunrise_world::loot::LootEntity {kind:"minecraft:player".into(),clones:Arc::new(AtomicUsize::new(0)),equipment:vec![]};
    assert_eq!(bridge::killer_mainhand_looting(&empty),0);
    let air=sunrise_world::loot::LootEntity {kind:"minecraft:player".into(),clones:Arc::new(AtomicUsize::new(0)),equipment:vec![
        ("mainhand".into(),sunrise_world::loot::LootStack {item:"minecraft:air".into(),count:0,enchantments:vec![("minecraft:looting".into(),3)]})]};
    assert_eq!(bridge::killer_mainhand_looting(&air),0);
}
