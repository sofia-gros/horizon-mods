// What the block behaviours of PlutoLib's API crate (block_api_v1) need from the game side. The registry of behaviours lives in the API crate;
// the injections inside the game cannot name it, so the API crate hands over these functions once when PlutoLib starts. Everything here is
// plain Rust: states travel as a block name and its properties, positions as arrays, faces as the game's direction numbers
// (0 down, 1 up, 2 north, 3 south, 4 west, 5 east).
use std::collections::BTreeMap;
use std::sync::Mutex;

/// A block state: the block's name and its properties.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StateInfo {
    pub block: String,
    pub properties: BTreeMap<String, String>,
}

/// What a behaviour can look at, with the game's own answers (not guesses).
pub trait LevelView {
    /// The state at `pos`; None where nothing is loaded.
    fn state(&self, pos: [i32; 3]) -> Option<StateInfo>;
    fn is_air(&self, pos: [i32; 3]) -> bool;
    /// Whether the whole `face` of the block at `pos` is solid (`BlockState.isFaceSturdy`).
    fn face_sturdy(&self, pos: [i32; 3], face: u8) -> bool;
    /// Whether the block at `pos` is in the block tag `tag` (`tconstruct:slimy_leaves`).
    fn in_tag(&self, pos: [i32; 3], tag: &str) -> bool;
}

/// A view that can also change the level.
pub trait LevelEdit: LevelView {
    /// Sets the block at `pos` to `state` with `Level.setBlock`'s flags; false if it could not be set.
    fn set_state(&mut self, pos: [i32; 3], state: &StateInfo, flags: i32) -> bool;
    /// A number from 0 up to but not including `bound`, from the level's own random source.
    fn random_int(&mut self, bound: i32) -> i32;
    /// Schedules a tick of `block` at `pos` after `delay` game ticks.
    fn schedule_tick(&mut self, pos: [i32; 3], block: &str, delay: i32) -> bool;
}

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

static SINK: Mutex<Option<Sink>> = Mutex::new(None);

pub fn set_sink(sink: Sink) {
    if let Ok(mut slot) = SINK.lock() {
        *slot = Some(sink);
    }
}

pub fn sink() -> Option<Sink> {
    SINK.lock().ok().and_then(|slot| *slot)
}
