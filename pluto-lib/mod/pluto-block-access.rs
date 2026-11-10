// Host-independent editor contract shared by the world and client adapters.
// Names/properties are owned; all world/RNG access remains borrowed.
use std::collections::BTreeMap;

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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StateInfo {
    pub block: String,
    pub properties: BTreeMap<String, String>,
}

pub trait LevelView {
    /// None when unavailable; never use a guessed default state.
    fn state(&self, pos: [i32; 3]) -> Option<StateInfo>;
    fn is_air(&self, pos: [i32; 3]) -> bool;
    /// Native faces: down/up/north/south/west/east = 0..5.
    fn face_sturdy(&self, pos: [i32; 3], face: u8) -> bool;
    fn in_tag(&self, pos: [i32; 3], tag: &str) -> bool;
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
