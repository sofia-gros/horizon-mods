// Host-independent editor contract shared by the world and client adapters.
// Names/properties are owned; all world/RNG access remains borrowed.
use std::collections::BTreeMap;

/// Landing transport lives below both client crates; callbacks are copied out
/// before invocation, just like the feature and survival dispatchers.
pub struct FallOn {
    pub state: StateInfo,
    pub pos: [i32; 3],
    pub entity: i32,
    pub kind: String,
    pub distance: f64,
    pub damage: f32,
}
pub struct AfterFall {
    pub pos: [i32; 3],
    pub entity: i32,
    pub kind: String,
    pub living: bool,
    pub item: bool,
    pub suppressing_bounce: bool,
    pub velocity: [f64; 3],
    pub fall_distance: f64,
    pub on_ground: bool,
}
pub struct InsideMotion {
    pub pos: [i32; 3],
    pub entity: i32,
    pub kind: String,
    pub server: bool,
    pub suppressing_bounce: bool,
    pub position: [f64; 3],
    pub previous_position: [f64; 3],
    pub velocity: [f64; 3],
    pub sync_velocity: bool,
    pub step_sound: Option<[f32; 2]>,
}
pub type InsideMotionHandler = std::sync::Arc<dyn Fn(&mut InsideMotion) + Send + Sync>;
pub type AfterFallHandler = std::sync::Arc<dyn Fn(&mut AfterFall) + Send + Sync>;
/// An entity is inside a block's space: the notification the movement seams hand to the API
/// crate, which turns it into the block behaviour's `entity_inside` callback. Read-only: the
/// seams run inside entity movement, where the entity may not be changed.
pub struct EntityInside {
    pub dimension: String,
    pub state: StateInfo,
    pub pos: [i32; 3],
    pub entity: i32,
    pub kind: String,
}
#[derive(Clone, Copy)]
pub struct LandingSink {
    pub has: fn(&str) -> bool,
    pub fall_on: fn(&mut FallOn),
    pub after_fall: fn(&StateInfo) -> Option<AfterFallHandler>,
    pub inside_motion: fn(&StateInfo) -> Option<InsideMotionHandler>,
    pub entity_inside: fn(&EntityInside),
}
static LANDING_SINK: std::sync::RwLock<Option<LandingSink>> = std::sync::RwLock::new(None);
pub fn set_landing_sink(sink: LandingSink) {
    *LANDING_SINK.write().expect("landing dispatcher poisoned") = Some(sink);
}
pub fn landing_sink() -> Option<LandingSink> {
    LANDING_SINK.read().ok().and_then(|slot| *slot)
}

/// Applies the block ticks host-side callbacks queued because the mod-facing host cannot
/// schedule (the API crate installs it; the level's tick seams call `drain_schedules` with
/// their own dimension and editor). Copy the function pointer out before invoking it.
pub type ScheduleDrain = fn(&str, &mut dyn LevelEdit);
static SCHEDULE_DRAIN: std::sync::RwLock<Option<ScheduleDrain>> = std::sync::RwLock::new(None);
pub fn set_schedule_drain(drain: ScheduleDrain) {
    if let Ok(mut slot) = SCHEDULE_DRAIN.write() { *slot = Some(drain) }
}
pub fn drain_schedules(dimension: &str, edit: &mut dyn LevelEdit) {
    let drain = SCHEDULE_DRAIN.read().ok().and_then(|slot| *slot);
    if let Some(drain) = drain { drain(dimension, edit); }
}

pub type FeatureDispatcher=fn(&mut dyn LevelEdit,&str,&serde_json::Value,[i32;3])->Option<bool>;
static FEATURE_DISPATCHER:std::sync::RwLock<Option<FeatureDispatcher>>=std::sync::RwLock::new(None);
pub fn set_feature_dispatcher(dispatcher:FeatureDispatcher) {
    if let Ok(mut slot)=FEATURE_DISPATCHER.write() {*slot=Some(dispatcher)}
}
/// Registered false/error results own the request; only None permits native
/// fallback. Copy the function pointer out before invoking any consumer code.
pub fn dispatch_feature(world:&mut dyn LevelEdit,kind:&str,configuration:&serde_json::Value,pos:[i32;3])->Result<Option<bool>,String> {
    let dispatcher=*FEATURE_DISPATCHER.read().map_err(|_|"feature dispatcher poisoned".to_owned())?;
    Ok(dispatcher.and_then(|dispatcher|dispatcher(world,kind,configuration,pos)))
}

pub type SurvivalDispatcher = fn(&dyn LevelView, &StateInfo, [i32; 3]) -> Option<bool>;
static SURVIVAL_DISPATCHER: std::sync::RwLock<Option<SurvivalDispatcher>> = std::sync::RwLock::new(None);
pub fn set_survival_dispatcher(dispatcher: SurvivalDispatcher) {
    *SURVIVAL_DISPATCHER.write().expect("plant survival dispatcher poisoned") = Some(dispatcher);
}
/// A registered false result must not fall through to vanilla survival. Invoke
/// callbacks without the bridge lock so registration and nested queries are safe.
pub fn dispatch_survival(world: &dyn LevelView, state: &StateInfo, pos: [i32; 3]) -> Result<Option<bool>, String> {
    let dispatcher = *SURVIVAL_DISPATCHER.read().map_err(|_| "plant survival dispatcher poisoned")?;
    Ok(dispatcher.and_then(|dispatcher| dispatcher(world, state, pos)))
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StateInfo {
    pub block: String,
    pub properties: BTreeMap<String, String>,
}

/// Borrowed path-node queries live below the executable and client library.
pub type PathNodeDispatcher = fn(&dyn LevelView, &StateInfo, [i32; 3], Option<(i32, &str)>) -> Option<String>;
static PATH_NODE_DISPATCHER: std::sync::RwLock<Option<PathNodeDispatcher>> = std::sync::RwLock::new(None);
pub fn set_path_node_dispatcher(dispatcher: PathNodeDispatcher) {
    *PATH_NODE_DISPATCHER.write().expect("path-node dispatcher poisoned") = Some(dispatcher);
}
pub fn dispatch_path_node(view: &dyn LevelView, state: &StateInfo, pos: [i32; 3], entity: Option<(i32, &str)>) -> Option<String> {
    let dispatcher = *PATH_NODE_DISPATCHER.read().ok()?;
    dispatcher.and_then(|dispatcher| dispatcher(view, state, pos, entity))
}

pub trait LevelView {
    /// None when unavailable; never use a guessed default state.
    fn state(&self, pos: [i32; 3]) -> Option<StateInfo>;
    fn is_air(&self, pos: [i32; 3]) -> bool;
    fn replaceable(&self, pos: [i32; 3]) -> bool { self.is_air(pos) }
    fn generation_depth(&self) -> Option<i32> { None }
    /// A real native Level.hasNeighborSignal query, not a state-property guess.
    fn has_neighbor_signal(&self, _pos: [i32; 3]) -> Option<bool> { None }
    /// Native faces: down/up/north/south/west/east = 0..5.
    fn face_sturdy(&self, pos: [i32; 3], face: u8) -> bool;
    fn in_tag(&self, pos: [i32; 3], tag: &str) -> bool;
    fn state_in_tag(&self, _state: &StateInfo, _tag: &str) -> Option<bool> { None }
    /// Combined sky/block brightness, not block light alone.
    fn brightness(&self, pos: [i32; 3]) -> u8;
    fn light_passes_up(&self, pos: [i32; 3]) -> bool;
    fn is_water(&self, pos: [i32; 3]) -> bool;
    fn fluid_full(&self, pos: [i32; 3]) -> bool;
    fn area_loaded(&self, pos: [i32; 3], radius: i32) -> bool;
    fn full_block(&self, pos: [i32; 3]) -> bool;
    /// Inclusive minimum, exclusive maximum build Y.
    fn build_height(&self) -> [i32; 2];
}

/// Gate an actual native power query on a valid, loaded origin. Native code
/// still owns neighbor/conductor/mod-hook semantics; do not reconstruct them
/// from a powered property, neighboring tags or a guessed circuit radius.
pub fn query_neighbor_signal(
    pos: [i32; 3], bounds: [i32; 2], loaded: impl FnOnce(i32, i32) -> bool,
    query: impl FnOnce() -> bool,
) -> Option<bool> {
    if bounds[0] >= bounds[1] || pos[1] < bounds[0] || pos[1] >= bounds[1]
        || !loaded(pos[0], pos[2]) { return None; }
    Some(query())
}

pub const MAX_FEATURE_DEPTH: u8 = 64;

/// Run with the native-style RNG copy and always write the advanced stream back,
/// including unwind. Completed world changes are not rolled back.
pub fn with_random<S, R: Clone, T>(state: &mut S, random: for<'a> fn(&'a mut S) -> &'a mut R, body: impl FnOnce(&mut S, &mut R) -> T) -> T {
    let mut copy = random(state).clone();
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| body(state, &mut copy)));
    *random(state) = copy;
    match result { Ok(value) => value, Err(panic) => std::panic::resume_unwind(panic) }
}

#[cfg(test)]
mod tests {
    #[test]
    fn neighbor_signal_gate_preserves_false_and_true_without_querying_unavailable_origins() {
        use super::query_neighbor_signal;
        use std::cell::Cell;
        let calls = Cell::new(0);
        for answer in [false, true] {
            let result = query_neighbor_signal([-17, 64, 31], [-64, 320], |x, z| {
                assert_eq!((x, z), (-17, 31)); true
            }, || { calls.set(calls.get() + 1); answer });
            assert_eq!(result, Some(answer));
        }
        assert_eq!(calls.get(), 2);
        for (pos, bounds, loaded) in [([0, -65, 0], [-64, 320], true),
            ([0, 320, 0], [-64, 320], true), ([0, 64, 0], [10, 10], true),
            ([0, 64, 0], [-64, 320], false)] {
            assert_eq!(query_neighbor_signal(pos, bounds, |_, _| loaded,
                || panic!("must not guess power at unavailable origin")), None);
        }
    }

    #[test]
    fn borrowed_block_queries_preserve_explicit_answers_native_fallback_and_unlocked_dispatch() {
        use super::*;
        struct View;
        impl LevelView for View {
            fn state(&self, pos: [i32; 3]) -> Option<StateInfo> {
                (pos == [0, 63, 0]).then(|| StateInfo { block: "example:soil".into(), properties: BTreeMap::new() })
            }
            fn is_air(&self, _: [i32; 3]) -> bool { unreachable!() }
            fn face_sturdy(&self, _: [i32; 3], _: u8) -> bool { unreachable!() }
            fn in_tag(&self, _: [i32; 3], _: &str) -> bool { unreachable!() }
            fn brightness(&self, _: [i32; 3]) -> u8 { unreachable!() }
            fn light_passes_up(&self, _: [i32; 3]) -> bool { unreachable!() }
            fn is_water(&self, _: [i32; 3]) -> bool { unreachable!() }
            fn fluid_full(&self, _: [i32; 3]) -> bool { unreachable!() }
            fn area_loaded(&self, _: [i32; 3], _: i32) -> bool { unreachable!() }
            fn full_block(&self, _: [i32; 3]) -> bool { unreachable!() }
            fn build_height(&self) -> [i32; 2] { unreachable!() }
        }
        fn survives(view: &dyn LevelView, state: &StateInfo, pos: [i32; 3]) -> Option<bool> {
            // Re-enter registration: holding the read lock here would deadlock.
            set_survival_dispatcher(survives);
            if state.block != "example:plant" { return None; }
            Some(view.state([pos[0], pos[1] - 1, pos[2]]).is_some_and(|s| s.block == "example:soil"))
        }
        let plant = StateInfo { block: "example:plant".into(), properties: BTreeMap::new() };
        assert_eq!(dispatch_survival(&View, &plant, [0, 64, 0]).unwrap(), None);
        set_survival_dispatcher(survives);
        assert_eq!(dispatch_survival(&View, &plant, [0, 64, 0]).unwrap(), Some(true));
        assert_eq!(dispatch_survival(&View, &plant, [1, 64, 0]).unwrap(), Some(false));
        let native = StateInfo { block: "minecraft:fern".into(), ..plant };
        assert_eq!(dispatch_survival(&View, &native, [0, 64, 0]).unwrap(), None);
        fn path(view: &dyn LevelView, state: &StateInfo, pos: [i32; 3], entity: Option<(i32, &str)>) -> Option<String> {
            set_path_node_dispatcher(path); // callback runs outside the bridge lock
            if state.block != "example:soil" { return None; }
            if entity.is_some() { return Some("BLOCKED".into()); }
            view.state(pos).map(|_| "STICKY_HONEY".into())
        }
        let soil = View.state([0, 63, 0]).unwrap();
        assert_eq!(dispatch_path_node(&View, &soil, [0, 63, 0], None), None);
        set_path_node_dispatcher(path);
        assert_eq!(dispatch_path_node(&View, &soil, [0, 63, 0], None).as_deref(), Some("STICKY_HONEY"));
        assert_eq!(dispatch_path_node(&View, &soil, [0, 63, 0], Some((4, "minecraft:cow"))).as_deref(), Some("BLOCKED"));
        assert_eq!(dispatch_path_node(&View, &soil, [1, 63, 0], None), None);
        assert_eq!(dispatch_path_node(&View, &native, [0, 63, 0], None), None);
    }

    #[test]
    fn tree_plans_require_each_native_once_and_preserve_interleaving() {
        use super::{TreeStep::{Native, Custom}, valid_tree_plan};
        assert!(valid_tree_plan(&[Custom(0), Native(1), Custom(1), Native(0), Custom(2)], 2));
        assert!(valid_tree_plan(&[], 0));
        assert!(!valid_tree_plan(&[Native(0), Native(0)], 2));
        assert!(!valid_tree_plan(&[Custom(0), Native(0)], 2));
        assert!(!valid_tree_plan(&[Native(2), Native(0)], 2));
        assert!(!valid_tree_plan(&vec![Custom(0); 513], 0));
        assert!(!valid_tree_plan(&[], 257));
    }

    #[test]
    fn rng_copy_is_written_back_on_success_and_caught_panic_without_world_rollback() {
        struct State { random: u64, writes: usize }
        let mut state = State { random: 5, writes: 0 };
        let value = super::with_random(&mut state, |s| &mut s.random, |s, r| { *r += 3; s.writes += 1; 17 });
        assert_eq!((value, state.random, state.writes), (17, 8, 1));
        let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            super::with_random(&mut state, |s| &mut s.random, |s, r| {
                *r += 7;
                s.writes += 1;
                std::panic::panic_any("original tree panic");
            });
        })).unwrap_err();
        assert_eq!(panic.downcast_ref::<&str>(), Some(&"original tree panic"));
        assert_eq!((state.random, state.writes), (15, 2));
    }
}

#[derive(Clone, Default)]
pub struct RootBindings {
    pub block_sets: Vec<serde_json::Value>,
    pub state_providers: Vec<serde_json::Value>,
}

pub trait RootEdit: LevelEdit {
    fn matches_block_set(&mut self, index: usize, pos: [i32; 3]) -> Result<bool, String>;
    fn sample_state_provider(&mut self, index: usize, pos: [i32; 3]) -> Result<StateInfo, String>;
}

pub type RootCallback<'a> = dyn FnMut(&mut dyn RootEdit, [i32; 3]) -> Result<Option<StateInfo>, String> + 'a;

/// Native indices refer to the decoded configuration, custom indices to callbacks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TreeStep { Native(usize), Custom(usize) }

/// Reject omissions, repeats and invalid native indices before any tree writes.
pub fn valid_tree_plan(plan: &[TreeStep], native_count: usize) -> bool {
    if plan.len() > 512 || native_count > 256 { return false }
    let mut seen = vec![false; native_count];
    for step in plan {
        if let TreeStep::Native(index) = *step {
            let Some(used) = seen.get_mut(index) else { return false };
            if *used { return false }
            *used = true;
        }
    }
    seen.into_iter().all(|used| used)
}

pub type PlannedTreeCallback<'a> = dyn FnMut(usize, &mut dyn LevelEdit, &[[i32; 3]], &[[i32; 3]], &[[i32; 3]]) -> Result<(), String> + 'a;

pub type TreeCallback<'a> = dyn FnMut(&mut dyn LevelEdit, &[[i32; 3]], &[[i32; 3]], &[[i32; 3]]) -> Result<(), String> + 'a;

pub trait LevelEdit: LevelView {
    /// Forward exact native update flags; false means the write could not occur.
    fn set_state(&mut self, pos: [i32; 3], state: &StateInfo, flags: i32) -> bool;
    /// Native destruction including fluid replacement and optional loot.
    /// None is unavailable; false is a supported call without a block change.
    fn break_block(&mut self, _pos: [i32; 3], _drop: bool) -> Option<bool> { None }
    /// All draws use the supplied placement stream, never a global/fresh RNG.
    fn random_int(&mut self, bound: i32) -> i32;
    fn random_float(&mut self) -> f32;
    fn schedule_tick(&mut self, pos: [i32; 3], block: &str, delay: i32) -> bool;
    fn place_feature(&mut self, feature: &str, pos: [i32; 3]) -> bool;
    /// Native tree followed by extra decorators, before native finalization.
    /// Unsupported editors fail without writing.
    fn place_tree(&mut self, _configuration: &serde_json::Value, _pos: [i32; 3], _decorate: &mut TreeCallback<'_>) -> bool { false }
    fn place_tree_with_roots(&mut self, _configuration: &serde_json::Value, _pos: [i32; 3], _bindings: &RootBindings, _root: &mut RootCallback<'_>, _decorate: &mut TreeCallback<'_>) -> bool { false }
    fn place_tree_with_plan(&mut self, _configuration: &serde_json::Value, _pos: [i32; 3], _roots: Option<(&RootBindings, &mut RootCallback<'_>)>, _plan: &[TreeStep], _decorate: &mut PlannedTreeCallback<'_>) -> bool { false }
}
