// What the block behaviours of PlutoLib's API crate (block_api_v1) need from the game side. The registry of behaviours lives in the API crate;
// the injections inside the game cannot name it, so the API crate hands over these functions once when PlutoLib starts. Everything here is
// plain Rust: states travel as a block name and its properties, positions as arrays, faces as the game's direction numbers
// (0 down, 1 up, 2 north, 3 south, 4 west, 5 east).
use std::sync::Mutex;

// Shared in the lower-level world crate so a native tree callback can use the
// same editor contract without introducing a world -> client dependency.
#[cfg(not(test))]
pub use horizon_world::level::pluto_worldgen_level::{StateInfo, LevelView, LevelEdit, MAX_FEATURE_DEPTH};

// Pure tests compile the same shared source, not a copied contract.
#[cfg(test)]
pub use crate::block_access_tests::{StateInfo, LevelView, LevelEdit, MAX_FEATURE_DEPTH};

pub struct ShapeUpdate {
    pub state: StateInfo,
    pub pos: [i32; 3],
    pub direction: u8,
    pub neighbour_pos: [i32; 3],
    pub neighbour: Option<StateInfo>,
}

pub struct Placement {
    pub state: StateInfo,
    pub pos: [i32; 3],
    pub face: u8,
    pub yaw: f32,
    pub pitch: f32,
    pub sneaking: bool,
    pub in_water: bool,
    pub item: String,
    pub replace_clicked: bool,
}

pub enum PlacementAnswer {
    Default,
    Refuse,
    State(StateInfo),
}

/// The functions the API crate gives the game.
#[derive(Clone, Copy)]
pub struct Sink {
    /// Whether a behaviour is registered for this block name.
    pub has: fn(&str) -> bool,
    pub survives: fn(&dyn LevelView, &StateInfo, [i32; 3]) -> Option<bool>,
    pub placement: fn(&dyn LevelView, &Placement) -> PlacementAnswer,
    pub update_shape: fn(&dyn LevelView, &ShapeUpdate) -> Option<StateInfo>,
    /// A random tick; true when a behaviour handled it (the game's own tick is skipped).
    pub random_tick: fn(&mut dyn LevelEdit, &StateInfo, [i32; 3]) -> bool,
    /// A scheduled tick; true when a behaviour handled it.
    pub scheduled_tick: fn(&mut dyn LevelEdit, &StateInfo, [i32; 3]) -> bool,
    pub bonemeal_valid: fn(&dyn LevelView, &StateInfo, [i32; 3]) -> Option<bool>,
    pub bonemeal_success: fn(&mut dyn LevelEdit, &StateInfo, [i32; 3]) -> Option<bool>,
    pub bonemeal_perform: fn(&mut dyn LevelEdit, &StateInfo, [i32; 3]),
    /// A plant, the ground below it and the ground's position.
    pub may_place_on: fn(&dyn LevelView, &StateInfo, &StateInfo, [i32; 3]) -> Option<bool>,
}

/// Scope only the recursion bookkeeping. Preserve completed writes and rethrow
/// the original panic; a consumer may catch it and keep using the same editor.
pub fn with_feature_depth<S>(state: &mut S, depth: fn(&mut S) -> &mut u8, body: impl FnOnce(&mut S) -> Option<bool>) -> Option<bool> {
    let previous = *depth(state);
    if previous >= MAX_FEATURE_DEPTH { return Some(false); }
    *depth(state) = previous + 1;
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| body(state)));
    *depth(state) = previous;
    match result {
        Ok(answer) => answer,
        Err(panic) => std::panic::resume_unwind(panic),
    }
}

#[cfg(test)]
mod feature_depth_tests {
    use super::*;
    use std::panic::{AssertUnwindSafe, catch_unwind};

    #[derive(Default)]
    struct State { depth: u8, calls: u32, writes: u32 }
    fn depth(state: &mut State) -> &mut u8 { &mut state.depth }
    fn recursive(state: &mut State) -> Option<bool> {
        with_feature_depth(state, depth, |state| {
            state.calls += 1;
            recursive(state)
        })
    }

    #[test]
    fn scopes_bound_recursion_and_restore_after_caught_panics_without_rollback() {
        let mut state = State::default();
        assert_eq!(recursive(&mut state), Some(false));
        assert_eq!(state.calls, u32::from(MAX_FEATURE_DEPTH));
        assert_eq!(state.depth, 0);
        assert_eq!(with_feature_depth(&mut state, depth, |state| {
            // More than the depth limit worth of sequential, caught inner panics
            // must not exhaust the still-active outer scope's recursion budget.
            for _ in 0..70 {
                let panic = catch_unwind(AssertUnwindSafe(|| with_feature_depth(state, depth, |state| {
                    state.writes += 1;
                    std::panic::panic_any("original feature panic");
                }))).unwrap_err();
                assert_eq!(panic.downcast_ref::<&str>(), Some(&"original feature panic"));
                assert_eq!(state.depth, 1);
            }
            Some(true)
        }), Some(true));
        assert_eq!(state.depth, 0);
        assert_eq!(state.writes, 70, "world-side effects are not rolled back");
    }
}

type FeatureDispatcher = fn(&mut dyn LevelEdit, &str, &serde_json::Value, [i32; 3]) -> Option<bool>;
static FEATURES: Mutex<Option<FeatureDispatcher>> = Mutex::new(None);

pub fn set_feature_dispatcher(dispatch: FeatureDispatcher) {
    if let Ok(mut slot) = FEATURES.lock() { *slot = Some(dispatch); }
}

/// None means no custom type was registered; retain native placement in that case.
pub fn custom_feature(world: &mut dyn LevelEdit, kind: &str, config: &serde_json::Value, pos: [i32; 3]) -> Option<bool> {
    let dispatch = *FEATURES.lock().ok()?;
    dispatch?(world, kind, config, pos)
}

static SINK: Mutex<Option<Sink>> = Mutex::new(None);

pub fn set_sink(sink: Sink) {
    if let Ok(mut slot) = SINK.lock() {
        *slot = Some(sink);
    }
}

pub fn sink() -> Option<Sink> {
    SINK.lock().ok().and_then(|slot| *slot)
}
