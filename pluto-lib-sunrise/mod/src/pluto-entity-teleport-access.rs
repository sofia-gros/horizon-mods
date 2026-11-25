// Player teleport transport for the entity reach: a teleport of a player
// cannot run inside the callback that asks for it (the game's server
// teleport call wants the level's own tick context), so requests queue
// here and `drain` — woven at the head of the game's own rider
// synchronisation — applies them through the game's own server teleport,
// the call /tp itself uses: the player leaves their vehicle, loses their
// motion and fall, and their client is told. An accepted request is
// therefore observable from the next tick, and the game's own bounds (a
// player on the server, a living one, the three dimensions, the /tp
// position bounds) keep the last word: a teleport they refuse simply
// never lands.
use super::Game;
use std::sync::Mutex;

/// One queued player teleport, applied by [`drain`] in the level it names.
#[derive(Clone, Copy, Debug)]
pub struct TeleportRequest {
    pub uuid: u128,
    pub position: [f64; 3],
    pub yaw: f32,
    pub pitch: f32,
}

static REQUESTS: Mutex<Vec<(String, TeleportRequest)>> = Mutex::new(Vec::new());

/// The most requests held for later levels; past it, new requests are refused.
const QUEUE_CAP: usize = 1024;

/// Queues a teleport of the player `uuid` to `position` in `dimension`, keeping the facing
/// `yaw`/`pitch` the request names (the caller's read of the player's current facing, as
/// `Entity.teleportTo` keeps it): true when the request was accepted. The game re-checks at
/// application time, so acceptance is a request taken, not a teleport guaranteed: read the
/// player's position from the next tick for the outcome.
pub fn request_teleport(dimension: &str, uuid: u128, position: [f64; 3], yaw: f32, pitch: f32) -> bool {
    let Ok(mut requests) = REQUESTS.lock() else { return false };
    if requests.len() >= QUEUE_CAP {
        return false;
    }
    requests.push((dimension.to_string(), TeleportRequest { uuid, position, yaw, pitch }));
    true
}

/// Applies this level's queued teleports through the game's own server teleport call.
pub fn drain(game: &mut Game<'_>) {
    let dimension = game.dim.id();
    let mut queued: Vec<TeleportRequest> = Vec::new();
    if let Ok(mut requests) = REQUESTS.lock() {
        let mut kept: Vec<(String, TeleportRequest)> = Vec::new();
        for (queued_dimension, request) in requests.drain(..) {
            if queued_dimension == dimension {
                queued.push(request);
            } else {
                kept.push((queued_dimension, request));
            }
        }
        *requests = kept;
    }
    for request in queued {
        game.native_teleport_player(request.uuid, dimension, request.position, request.yaw, request.pitch);
    }
}
