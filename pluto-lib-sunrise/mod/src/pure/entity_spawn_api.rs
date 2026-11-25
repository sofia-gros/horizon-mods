//! Split-phase mob finalization: registered preparations run before inherited
//! initialization; owned completion work runs once after ALL inherited tails.
//! No callback-local entity ID, world pointer or re-read of current registrations
//! is retained in the prepared work. Native attachment remains source-guarded.
use super::{base::Identifier,player_api::StackData};
use std::sync::{Arc,Mutex};

/// Stable spawn metadata, not a handle to an already published entity.
#[derive(Clone,Debug,PartialEq)]
pub struct MobSpawnInfo {
    pub entity_type:Identifier,
    pub dimension:Identifier,
    pub position:[f64;3],
    /// Backend reason, e.g. natural, command, spawner, structure or conversion.
    pub reason:String,
    /// The local difficulty's special multiplier at the position
    /// (`DifficultyInstance.getSpecialMultiplier`): 0 below effective difficulty
    /// 2, 1 above 4, linear between. Every difficulty-scaled finalize roll of
    /// the original (pickup loot, class equipment) reads this one value.
    pub special_multiplier:f32,
}
/// Borrowed preparation view. RNG is the new mob's stream, NOT the world's
/// independent enchantment/spawner stream. Gear reads are owned copies.
pub trait MobSpawnPreparation {
    fn info(&self)->&MobSpawnInfo;
    fn next_float(&mut self)->f32;
    fn next_double(&mut self)->f64;
    fn next_int(&mut self,bound:i32)->Result<i32,String>;
    fn equipment(&self,slot:&str)->Result<Option<StackData>,String>;
}
/// Post-inherited-initialization view of the same unpublished mob. Unsupported
/// slots/components reject explicitly; setting gear is not inherited finalization.
pub trait MobSpawnCompletion:MobSpawnPreparation {
    fn set_equipment(&mut self,slot:&str,stack:StackData)->Result<(),String>;
    /// `Mob.setCanPickUpLoot`: whether the mob may pick up dropped items.
    /// The pickup roll itself is the callback's, drawn during preparation.
    fn set_can_pick_up_loot(&mut self,_can:bool)->Result<(),String> { Err("mob spawn backend cannot set the pickup-loot flag".into()) }
    /// Record one equipment slot's death-drop chance (`Mob.setDropChance`):
    /// the Halloween pumpkin's 0, a picked-up item's 2, the default 0.085.
    fn set_drop_chance(&mut self,_slot:&str,_chance:f32)->Result<(),String> { Err("mob spawn backend cannot set equipment drop chances".into()) }
    /// Enchant the stack in `slot` as spawned armor (`Mob.enchantSpawnedArmor`):
    /// the level is the callback's roll, drawn during preparation; the backend
    /// runs the game's own enchantment selection at that level on the mob's
    /// stream. An empty slot is left alone.
    fn enchant_equipment(&mut self,_slot:&str,_level:i32)->Result<(),String> { Err("mob spawn backend cannot enchant spawn equipment".into()) }
    /// Store attachment data on the still-unpublished mob, under the same
    /// attachment id/JSON/sync policy the data-attachment API uses; the write
    /// lands before the mob joins, so load-time readers see it. The typed
    /// registry is not reachable here: serialize the value as the consumer's
    /// attachment type would.
    fn set_attached(&mut self,_attachment:&str,_json:&str,_synced:bool)->Result<(),String> { Err("mob spawn backend cannot store attachments".into()) }
}
/// Owned once-only work produced by a preparation callback. Captured resource
/// generations are released on completion, error or aborted/unpublished spawn.
pub type MobSpawnAction=Box<dyn FnOnce(&mut dyn MobSpawnCompletion)->Result<(),String>+Send+Sync>;
type Prepare=dyn Fn(&mut dyn MobSpawnPreparation)->Result<Option<MobSpawnAction>,String>+Send+Sync;
#[derive(Default)]
struct Registry { callbacks:Vec<(Identifier,Arc<Prepare>)> }
impl Registry {
    fn register(&mut self,id:Identifier,prepare:Arc<Prepare>)->Result<(),String> {
        if self.callbacks.iter().any(|(old,_)|old==&id) { return Err("duplicate mob spawn callback".into()); }
        self.callbacks.push((id,prepare));
        Ok(())
    }
    fn snapshot(&self)->Vec<Arc<Prepare>> { self.callbacks.iter().map(|(_,p)|p.clone()).collect() }
}
static REGISTRY:Mutex<Registry>=Mutex::new(Registry { callbacks:Vec::new() });
/// Registers general mob-finalization callbacks. Registration order is dispatch
/// order; a preparation returns None when it has no completion work, not a request
/// to roll back its RNG draws. Do not use entity LOAD for this lifecycle.
pub struct MobSpawnCallback;
impl MobSpawnCallback {
    /// Register once under an ID, without replacement. Preparation has no gear
    /// writer; return owned work for post-initialization gear edits. A callback
    /// added during preparation is considered only on subsequent spawns.
    pub fn register(id:Identifier,prepare:impl Fn(&mut dyn MobSpawnPreparation)->Result<Option<MobSpawnAction>,String>+Send+Sync+'static)->Result<(),String> {
        REGISTRY.lock().map_err(|_|"mob spawn callback registry poisoned")?.register(id,Arc::new(prepare))
    }
}
/// Native-owned pending dispatch. Not Clone or serializable; native Mob snapshots
/// must discard it, while moving the original Mob retains it.
pub(crate) struct PreparedSpawn { actions:Vec<MobSpawnAction> }
impl PreparedSpawn {
    pub(crate) fn complete(self,host:&mut dyn MobSpawnCompletion)->Result<(),String> {
        for action in self.actions { action(host)?; }
        Ok(())
    }
}
fn prepare_callbacks(callbacks:Vec<Arc<Prepare>>,host:&mut dyn MobSpawnPreparation)->Result<Option<PreparedSpawn>,String> {
    let mut actions=Vec::new();
    for callback in callbacks { if let Some(action)=callback(host)? { actions.push(action); } }
    Ok((!actions.is_empty()).then_some(PreparedSpawn { actions }))
}
pub(crate) fn has_callbacks()->Result<bool,String> {
    Ok(!REGISTRY.lock().map_err(|_|"mob spawn callback registry poisoned")?.callbacks.is_empty())
}
pub(crate) fn prepare(host:&mut dyn MobSpawnPreparation)->Result<Option<PreparedSpawn>,String> {
    let callbacks=REGISTRY.lock().map_err(|_|"mob spawn callback registry poisoned")?.snapshot();
    // Execute mod code and drop its captured resources outside registry locks.
    prepare_callbacks(callbacks,host)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{collections::VecDeque,sync::atomic::{AtomicUsize,Ordering}};
    struct Host { info:MobSpawnInfo,draws:VecDeque<f32>,gear:StackData,trace:Vec<&'static str> }
    impl MobSpawnPreparation for Host {
        fn info(&self)->&MobSpawnInfo { &self.info }
        fn next_float(&mut self)->f32 { self.trace.push("chance"); self.draws.pop_front().unwrap() }
        fn next_double(&mut self)->f64 { panic!("unexpected double") }
        fn next_int(&mut self,_:i32)->Result<i32,String> { panic!("unexpected integer") }
        fn equipment(&self,_:&str)->Result<Option<StackData>,String> { Ok(Some(self.gear.clone())) }
    }
    impl MobSpawnCompletion for Host {
        fn set_equipment(&mut self,_:&str,stack:StackData)->Result<(),String> { self.trace.push("replace"); self.gear=stack; Ok(()) }
        fn set_can_pick_up_loot(&mut self,_:bool)->Result<(),String> { self.trace.push("pickup"); Ok(()) }
        fn set_drop_chance(&mut self,_:&str,_:f32)->Result<(),String> { self.trace.push("chance"); Ok(()) }
        fn enchant_equipment(&mut self,_:&str,_:i32)->Result<(),String> { self.trace.push("enchant"); Ok(()) }
        fn set_attached(&mut self,_:&str,_:&str,_:bool)->Result<(),String> { self.trace.push("attach"); Ok(()) }
    }
    #[test]
    fn dispatch_keeps_preparation_before_full_initialization_and_retains_owned_actions_until_once_only_completion() {
        let mut registry=Registry::default();
        let live=Arc::new(AtomicUsize::new(0));
        struct Lease(Arc<AtomicUsize>);
        impl Drop for Lease { fn drop(&mut self) { self.0.fetch_sub(1,Ordering::SeqCst); } }
        let captured=live.clone();
        registry.register(Identifier::parse("test:replace").unwrap(),Arc::new(move |host| {
            if host.next_float()>=0.5 { return Ok(None); }
            captured.fetch_add(1,Ordering::SeqCst);
            let lease=Lease(captured.clone());
            Ok(Some(Box::new(move |host:&mut dyn MobSpawnCompletion| {
                let _lease=lease;
                assert_eq!(host.equipment("mainhand")?.unwrap().item,"minecraft:enchanted_crossbow");
                assert_eq!(host.info().special_multiplier,0.5);
                host.set_can_pick_up_loot(true)?;
                host.set_drop_chance("head",0.0)?;
                host.enchant_equipment("head",5)?;
                host.set_attached("test:flag","true",true)?;
                host.set_equipment("mainhand",StackData { item:"test:replacement".into(),count:1,..Default::default() })
            }) as MobSpawnAction))
        })).unwrap();
        let mut host=Host { info:MobSpawnInfo { entity_type:Identifier::parse("minecraft:piglin").unwrap(),
            dimension:Identifier::parse("minecraft:overworld").unwrap(),position:[0.0;3],reason:"natural".into(),special_multiplier:0.5 },
            draws:[0.0,1.0].into(),gear:StackData::default(),trace:vec![] };
        let pending=prepare_callbacks(registry.snapshot(),&mut host).unwrap().unwrap();
        assert_eq!(live.load(Ordering::SeqCst),1);
        host.trace.extend(["default_gear","enchantment","seasonal","class_tail"]);
        host.gear.item="minecraft:enchanted_crossbow".into();
        pending.complete(&mut host).unwrap();
        assert_eq!(host.trace,["chance","default_gear","enchantment","seasonal","class_tail","pickup","chance","enchant","attach","replace"]);
        assert_eq!(live.load(Ordering::SeqCst),0);
        assert!(prepare_callbacks(registry.snapshot(),&mut host).unwrap().is_none());
        assert!(host.draws.is_empty(),"unsuccessful selection still consumes its pre-initialization draw");
        host.draws.push_back(0.0);
        let aborted=prepare_callbacks(registry.snapshot(),&mut host).unwrap();
        drop(aborted);
        assert_eq!(live.load(Ordering::SeqCst),0);
        assert!(registry.register(Identifier::parse("test:replace").unwrap(),Arc::new(|_|Ok(None))).is_err());
    }
}
