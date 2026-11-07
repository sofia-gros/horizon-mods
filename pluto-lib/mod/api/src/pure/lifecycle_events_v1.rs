//! Counterpart of Fabric's `fabric-lifecycle-events-v1`: server, world and chunk lifecycle events.
//!
//! Callbacks get reduced, owned data (Fabric passes server, world and chunk objects; PlutoLib starts with what Horizon's hook points report).

use super::base::Event;

/// What the server lifecycle callbacks learn about the server.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServerInfo {
    /// True for a dedicated server, false for the integrated one.
    pub dedicated: bool,
}

/// A world (dimension) tick.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorldTick {
    /// The dimension id, such as `minecraft:overworld`.
    pub dimension: String,
}

/// A chunk of a world.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServerChunk {
    pub dimension: String,
    pub x: i32,
    pub z: i32,
    /// True on `CHUNK_LOAD` when the chunk was generated now rather than read from disk.
    pub newly_generated: bool,
}

static SERVER_STARTING: Event<ServerInfo, ()> = Event::void();
static SERVER_STARTED: Event<ServerInfo, ()> = Event::void();
static SERVER_STOPPING: Event<ServerInfo, ()> = Event::void();
static SERVER_STOPPED: Event<ServerInfo, ()> = Event::void();

/// Fabric: `ServerLifecycleEvents`.
pub struct ServerLifecycleEvents;

impl ServerLifecycleEvents {
    /// Fabric: `SERVER_STARTING`.
    pub const SERVER_STARTING: &'static Event<ServerInfo, ()> = &SERVER_STARTING;
    /// Fabric: `SERVER_STARTED`. Stub: Horizon has no matching hook point yet (see the support matrix).
    pub const SERVER_STARTED: &'static Event<ServerInfo, ()> = &SERVER_STARTED;
    /// Fabric: `SERVER_STOPPING`.
    pub const SERVER_STOPPING: &'static Event<ServerInfo, ()> = &SERVER_STOPPING;
    /// Fabric: `SERVER_STOPPED`. Stub: Horizon has no matching hook point yet.
    pub const SERVER_STOPPED: &'static Event<ServerInfo, ()> = &SERVER_STOPPED;
}

static START_SERVER_TICK: Event<(), ()> = Event::void();
static END_SERVER_TICK: Event<(), ()> = Event::void();
static START_WORLD_TICK: Event<WorldTick, ()> = Event::void();
static END_WORLD_TICK: Event<WorldTick, ()> = Event::void();

/// Fabric: `ServerTickEvents`.
pub struct ServerTickEvents;

impl ServerTickEvents {
    pub const START_SERVER_TICK: &'static Event<(), ()> = &START_SERVER_TICK;
    pub const END_SERVER_TICK: &'static Event<(), ()> = &END_SERVER_TICK;
    pub const START_WORLD_TICK: &'static Event<WorldTick, ()> = &START_WORLD_TICK;
    pub const END_WORLD_TICK: &'static Event<WorldTick, ()> = &END_WORLD_TICK;
}

static CHUNK_LOAD: Event<ServerChunk, ()> = Event::void();
static CHUNK_UNLOAD: Event<ServerChunk, ()> = Event::void();

/// Fabric: `ServerChunkEvents`.
pub struct ServerChunkEvents;

impl ServerChunkEvents {
    pub const CHUNK_LOAD: &'static Event<ServerChunk, ()> = &CHUNK_LOAD;
    pub const CHUNK_UNLOAD: &'static Event<ServerChunk, ()> = &CHUNK_UNLOAD;
}

// The dispatchers below are what the Horizon hook translation (hooks.rs) calls.

pub(crate) fn server_starting(dedicated: bool) {
    SERVER_STARTING.invoke(&ServerInfo { dedicated });
}

pub(crate) fn server_stopping(dedicated: bool) {
    SERVER_STOPPING.invoke(&ServerInfo { dedicated });
}

pub(crate) fn server_tick(start: bool) {
    if start {
        START_SERVER_TICK.invoke(&());
    } else {
        END_SERVER_TICK.invoke(&());
    }
}

pub(crate) fn world_tick(start: bool, dimension: &str) {
    let tick = WorldTick { dimension: dimension.to_owned() };
    if start {
        START_WORLD_TICK.invoke(&tick);
    } else {
        END_WORLD_TICK.invoke(&tick);
    }
}

pub(crate) fn chunk_loaded(dimension: &str, chunk: [i32; 2], newly_generated: bool) {
    CHUNK_LOAD.invoke(&ServerChunk { dimension: dimension.to_owned(), x: chunk[0], z: chunk[1], newly_generated });
}

pub(crate) fn chunk_unloaded(dimension: &str, chunk: [i32; 2]) {
    CHUNK_UNLOAD.invoke(&ServerChunk { dimension: dimension.to_owned(), x: chunk[0], z: chunk[1], newly_generated: false });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    #[test]
    fn world_tick_reaches_start_or_end_with_the_dimension() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let (a, b) = (Arc::clone(&log), Arc::clone(&log));
        ServerTickEvents::START_WORLD_TICK.register(move |t| a.lock().unwrap().push(format!("start {}", t.dimension)));
        ServerTickEvents::END_WORLD_TICK.register(move |t| b.lock().unwrap().push(format!("end {}", t.dimension)));
        world_tick(true, "minecraft:the_nether");
        world_tick(false, "minecraft:the_nether");
        assert_eq!(*log.lock().unwrap(), ["start minecraft:the_nether", "end minecraft:the_nether"]);
    }

    #[test]
    fn chunk_events_carry_coordinates_and_generation() {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let s = Arc::clone(&seen);
        ServerChunkEvents::CHUNK_LOAD.register(move |c| s.lock().unwrap().push((c.x, c.z, c.newly_generated)));
        chunk_loaded("minecraft:overworld", [3, -4], true);
        assert_eq!(*seen.lock().unwrap(), [(3, -4, true)]);
    }
}
