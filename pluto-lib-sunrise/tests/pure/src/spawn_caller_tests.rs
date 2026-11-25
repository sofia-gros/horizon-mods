//! Executes the actual spawner publication helper against a trace host. This
//! checks caller order/abort ownership, not Horizon types or native attachment.
use crate::entity_spawn_state_tests::SpawnState;
use std::{collections::HashMap,marker::PhantomData};
macro_rules! caller_error { ($($arg:tt)*) => { eprintln!($($arg)*) }; }
mod log { pub(crate) use caller_error as error; }
#[derive(Clone,Default)]
struct Navigation { state:SpawnState }
impl Navigation {
    fn __horizon_mixins_pluto_lib_pluto_entity_spawn_state(&self)->&SpawnState { &self.state }
}
#[derive(Clone,Default)]
struct Ai { nav:Navigation }
#[derive(Clone)]
struct Mob { id:u32,ai:Ai,gear:u8 }
enum Planned { Finalized(Mob,Option<u8>),Loaded(u8,Option<u8>) }
#[derive(Default)]
pub(crate) struct Game<'a> { mobs:Vec<Mob>,next_entity:u32,trace:Vec<&'static str>,_life:PhantomData<&'a ()> }
impl Game<'_> {
    fn native_mobs_joined(&mut self,_:usize,_:bool) { self.trace.push("join"); }
    fn enchant_spawn_equipment(&mut self,_:u32) { self.trace.push("enchant"); }
    fn halloween_spawn_head(&mut self,_:u32) { self.trace.push("seasonal"); }
    fn spawn_jockeys(&mut self,_:u32,_:bool) { self.trace.push("jockey"); }
    fn load_mob(&mut self,tag:u8,_:Option<u32>)->Option<u32> {
        self.trace.push("load"); self.next_entity+=1;
        self.mobs.push(Mob { id:self.next_entity,ai:Ai::default(),gear:tag });
        Some(self.next_entity)
    }
    fn equip_from_table(&mut self,id:u32,gear:&u8) {
        self.trace.push("explicit_equipment");
        self.mobs.iter_mut().find(|mob|mob.id==id).unwrap().gear=*gear;
    }
}
pub(crate) mod bridge {
    use super::*;
    pub(crate) struct Prepared { succeed:bool,token:crate::entity_spawn_state_tests::SpawnToken }
    pub(super) fn take(mob:&Mob)->Result<Option<Prepared>,String> {
        mob.ai.nav.state.take_with_token::<bool>().map(|value|value.map(|(succeed,token)|Prepared { succeed,token }))
    }
    pub(super) fn complete(game:&mut Game<'_>,id:u32,prepared:Prepared)->Result<(),String> {
        let index=game.mobs.iter().position(|mob|mob.id==id).ok_or("missing mob")?;
        if !game.mobs[index].ai.nav.state.owns(&prepared.token)? { return Err("wrong owner".into()); }
        game.trace.push("complete");
        if !prepared.succeed { game.mobs.remove(index); return Err("refused gear".into()); }
        game.mobs[index].gear=7;
        Ok(())
    }
}
include!("../../../mod/pluto-entity-spawn-spawner-add.rs");
include!("../../../mod/pluto-entity-spawn-converted-head.rs");
fn mob(selected:Option<bool>)->Mob {
    let state=SpawnState::default();
    if let Some(succeed)=selected { state.prepare(succeed).unwrap(); }
    Mob { id:0,ai:Ai { nav:Navigation { state } },gear:1 }
}
#[test]
fn actual_converted_tail_skips_consumed_state_only_and_never_infers_it_from_entity_id() {
    use std::ops::ControlFlow;
    let mut game=Game::default();
    let mut original=mob(Some(true)); original.id=1;
    game.mobs.push(original);
    let snapshot=game.mobs[0].clone();
    assert!(snapshot.ai.nav.state.take::<bool>().unwrap().is_none(),"deep Mob/AI/navigation clones discard pending work");
    assert_eq!(pluto_entity_spawn_converted(&game,1),ControlFlow::Continue(()));
    let prepared=bridge::take(&game.mobs[0]).unwrap().unwrap();
    bridge::complete(&mut game,1,prepared).unwrap();
    assert_eq!(pluto_entity_spawn_converted(&game,1),ControlFlow::Break(()));
    assert_eq!(pluto_entity_spawn_converted(&game,2),ControlFlow::Continue(()));
    game.mobs[0]=Mob { id:1,ai:Ai::default(),gear:0 };
    assert_eq!(pluto_entity_spawn_converted(&game,1),ControlFlow::Continue(()));
}
#[test]
fn actual_spawner_caller_completes_after_inherited_tails_before_join_and_spawn_data_override() {
    let mut game=Game::default();
    let ids=pluto_entity_spawn_spawner_add(&mut game,vec![
        Planned::Finalized(mob(Some(true)),Some(9)),
        Planned::Finalized(mob(None),None),
        Planned::Loaded(4,Some(5)),
    ]);
    assert_eq!(game.trace,["enchant","seasonal","jockey","complete","join","explicit_equipment",
        "join","enchant","seasonal","jockey","load","explicit_equipment"]);
    assert_eq!(game.mobs.iter().map(|mob|mob.gear).collect::<Vec<_>>(),[9,1,5]);
    assert_eq!(ids.get(&u32::MAX),Some(&1));
    assert_eq!(ids.get(&(u32::MAX-1)),Some(&2));
    assert_eq!(ids.get(&(u32::MAX-2)),Some(&3));
    assert!(game.mobs[0].ai.nav.state.take::<bool>().unwrap().is_none());
}
#[test]
fn actual_spawner_caller_never_joins_or_publishes_failed_completion_but_keeps_later_candidates() {
    let mut game=Game::default();
    let ids=pluto_entity_spawn_spawner_add(&mut game,vec![
        Planned::Finalized(mob(Some(false)),Some(9)),Planned::Finalized(mob(Some(true)),None),
    ]);
    assert_eq!(game.trace,["enchant","seasonal","jockey","complete",
        "enchant","seasonal","jockey","complete","join"]);
    assert!(!ids.contains_key(&u32::MAX));
    assert_eq!(ids.get(&(u32::MAX-1)),Some(&2));
    assert_eq!(game.mobs.len(),1);
    assert_eq!((game.mobs[0].id,game.mobs[0].gear),(2,7));
}
