// Transport for a projectile's spawn and flight tellings: the game's projectile tick tells no mod API (the entity-tick
// telling covers mobs, items, minecarts and boats, API 146), so PlutoLib's reaches into the game's own projectile list
// (pluto-projectile-flight-game, in the game's creatures module) ask the API crate through this sink, in plain data.
// The facts go one way and the outcome comes back; world and entity work a flight asks for rides back as actions the
// game side does as the tick settles, because the telling runs outside the game's hook scope.
use std::sync::OnceLock;

/// An arrow's facts, as the game side reports them.
#[derive(Clone, Copy, Debug)]
pub struct ArrowFacts {
    pub pierce: u8,
    pub in_ground: bool,
    pub in_ground_time: u32,
    pub dealt_damage: bool,
}

/// A projectile joining the flying list, before its first tick. `projectile` is 0 when the game has not numbered it
/// yet (it is numbered as it joins, after this pass).
#[derive(Clone, Debug)]
pub struct SpawnFacts {
    pub dimension: String,
    pub projectile: u32,
    pub kind: String,
    pub owner: i64,
    pub position: [f64; 3],
    pub motion: [f64; 3],
    pub arrow: Option<ArrowFacts>,
    pub stack: Option<crate::items::ItemStack>,
}

/// What the API side settled for a spawn: the edits the game makes to the projectile before it flies.
#[derive(Clone, Debug, Default)]
pub struct SpawnOutcome {
    pub motion: Option<[f64; 3]>,
    pub pierce: Option<u8>,
    pub no_gravity: Option<bool>,
    pub pickup_stack: Option<crate::items::ItemStack>,
}

/// One thing a flight asks the game to do around a projectile, in plain data.
#[derive(Clone, Debug)]
pub enum FlightAction {
    /// `blast`: 0 keeps blocks, 1 breaks them by the block rule, 2 by the tnt rule.
    Explode { center: [f64; 3], radius: f32, fire: bool, blast: u8 },
    Particle { name: String, position: [f64; 3], count: u32, spread: [f64; 3], speed: f64 },
    PushEntity { entity: u32, delta: [f64; 3] },
    PullItems { center: [f64; 3], radius: f64, strength: f64 },
    PullEntities { center: [f64; 3], radius: f64, strength: f64 },
    DropStack { stack: crate::items::ItemStack, position: [f64; 3] },
    GiveStack { stack: crate::items::ItemStack },
}

/// A projectile at the end of a tick it flew (or rested stuck) in.
#[derive(Clone, Debug)]
pub struct FlightFacts {
    pub dimension: String,
    pub projectile: u32,
    pub kind: String,
    pub owner: i64,
    pub owner_position: Option<[f64; 3]>,
    pub age: u32,
    pub position: [f64; 3],
    pub motion: [f64; 3],
    pub in_ground: bool,
    pub no_gravity: bool,
    pub arrow: Option<ArrowFacts>,
    pub stack: Option<crate::items::ItemStack>,
}

/// What the API side settled for a flight tick.
#[derive(Clone, Debug, Default)]
pub struct FlightOutcome {
    pub motion: Option<[f64; 3]>,
    pub discard: bool,
    pub no_gravity: Option<bool>,
    pub release_from_ground: bool,
    pub actions: Vec<FlightAction>,
}

pub struct Sink {
    pub has_spawn: fn() -> bool,
    pub has_flight: fn() -> bool,
    pub spawn: fn(&SpawnFacts) -> SpawnOutcome,
    pub flight: fn(&FlightFacts) -> FlightOutcome,
}

static SINK: OnceLock<Sink> = OnceLock::new();

pub fn set_sink(sink: Sink) -> Result<(), &'static str> {
    SINK.set(sink).map_err(|_| "projectile flight sink already installed")
}

/// Whether anyone on the API side listens to spawns, so the game only builds the facts when someone does.
pub fn hears_spawn() -> bool {
    SINK.get().is_some_and(|sink| (sink.has_spawn)())
}

/// Whether anyone on the API side listens to flights, so the game only builds the facts when someone does.
pub fn hears_flight() -> bool {
    SINK.get().is_some_and(|sink| (sink.has_flight)())
}

pub fn spawn(facts: &SpawnFacts) -> Option<SpawnOutcome> {
    SINK.get().map(|sink| (sink.spawn)(facts))
}

pub fn flight(facts: &FlightFacts) -> Option<FlightOutcome> {
    SINK.get().map(|sink| (sink.flight)(facts))
}
