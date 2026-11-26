// Passenger (riding) transport for the entity reach: the pure API's ride
// questions and requests cross to the game here. The host adapter keeps a
// snapshot of who rides whom in the level it last ticked, and queues mount
// and dismount requests; `drain` — woven at the head of the game's own rider
// synchronisation — applies the queue through the game's own mount paths and
// rebuilds the snapshot. An accepted request is therefore observable from the
// next tick, and the game's own mount event, capacity and seating rules keep
// the last word: a mount they refuse simply never appears in the snapshot.
//
// The local player can be a vehicle (that is what carrying a mob means); the
// seat helper woven into `Game::seat` answers for the player, so the rider
// synchronisation keeps and seats such a mount. The local player as a
// passenger is the game's own boarding (it boards through its seat logic,
// with a teleport), so requests naming the player as the passenger are
// refused here rather than queued to fail.
use super::Game;
use std::sync::Mutex;

/// One queued change, applied by [`drain`] in the level it names.
#[derive(Clone, Copy, Debug)]
pub enum RideRequest {
    Mount { passenger: u32, vehicle: u32 },
    Dismount { passenger: u32 },
}

#[derive(Default)]
struct Snapshot {
    dimension: String,
    local_player: Option<u32>,
    /// `(passenger, vehicle)` pairs, in the order the snapshot read them.
    pairs: Vec<(u32, u32)>,
}

static REQUESTS: Mutex<Vec<(String, RideRequest)>> = Mutex::new(Vec::new());
static SNAPSHOT: Mutex<Snapshot> = Mutex::new(Snapshot { dimension: String::new(), local_player: None, pairs: Vec::new() });

/// The most requests held for later levels; past it, new requests are refused.
const QUEUE_CAP: usize = 1024;

fn vehicle_in(pairs: &[(u32, u32)], passenger: u32) -> Option<u32> {
    pairs.iter().find(|(rider, _)| *rider == passenger).map(|(_, vehicle)| *vehicle)
}

/// The vehicle `passenger` rides in `dimension`, from the last snapshot; `None` when it rides nothing, when the
/// snapshot is of another level, or when no snapshot exists yet.
pub fn vehicle_of(dimension: &str, passenger: u32) -> Option<u32> {
    let snapshot = SNAPSHOT.lock().ok()?;
    if snapshot.dimension != dimension {
        return None;
    }
    vehicle_in(&snapshot.pairs, passenger)
}

/// The direct passengers of `vehicle` in `dimension`, from the last snapshot, in the order they mounted.
pub fn passengers_of(dimension: &str, vehicle: u32) -> Vec<u32> {
    let Ok(snapshot) = SNAPSHOT.lock() else { return Vec::new() };
    if snapshot.dimension != dimension {
        return Vec::new();
    }
    snapshot.pairs.iter().filter(|(_, carrier)| *carrier == vehicle).map(|(passenger, _)| *passenger).collect()
}

/// Queues a mount of `passenger` on `vehicle` in `dimension`: true when the request was accepted, judged against
/// the last snapshot — the same entity twice, the local player as passenger, a passenger that already rides
/// something, and a cycle (the vehicle already rides the passenger, directly or through a chain) are refused at
/// once. The game re-checks at application time (its mount event and capacity), so acceptance is a request taken,
/// not a mount guaranteed: read [`vehicle_of`] from the next tick for the outcome.
pub fn request_mount(dimension: &str, passenger: u32, vehicle: u32) -> bool {
    if passenger == vehicle {
        return false;
    }
    {
        let Ok(snapshot) = SNAPSHOT.lock() else { return false };
        if snapshot.dimension != dimension || snapshot.local_player == Some(passenger) || vehicle_in(&snapshot.pairs, passenger).is_some() {
            return false;
        }
        let mut carrier = vehicle;
        while let Some(next) = vehicle_in(&snapshot.pairs, carrier) {
            if next == passenger {
                return false;
            }
            carrier = next;
        }
    }
    let Ok(mut requests) = REQUESTS.lock() else { return false };
    if requests.len() >= QUEUE_CAP {
        return false;
    }
    requests.push((dimension.to_string(), RideRequest::Mount { passenger, vehicle }));
    true
}

/// Queues a dismount of `passenger` in `dimension`: true when the last snapshot shows it riding something.
pub fn request_dismount(dimension: &str, passenger: u32) -> bool {
    {
        let Ok(snapshot) = SNAPSHOT.lock() else { return false };
        if snapshot.dimension != dimension || vehicle_in(&snapshot.pairs, passenger).is_none() {
            return false;
        }
    }
    let Ok(mut requests) = REQUESTS.lock() else { return false };
    if requests.len() >= QUEUE_CAP {
        return false;
    }
    requests.push((dimension.to_string(), RideRequest::Dismount { passenger }));
    true
}

/// Applies this level's queued requests through the game's own mount paths, then rebuilds the snapshot from the
/// level as it now stands. Runs at the head of the game's rider synchronisation, so a mount applied here is
/// validated and seated by that same synchronisation.
pub fn drain(game: &mut Game<'_>) {
    let dimension = game.dim.id();
    let mut queued: Vec<RideRequest> = Vec::new();
    if let Ok(mut requests) = REQUESTS.lock() {
        let mut kept: Vec<(String, RideRequest)> = Vec::new();
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
        match request {
            RideRequest::Mount { passenger, vehicle } => {
                if game.mobs.iter().any(|mob| mob.id == passenger) {
                    // The game's own startRiding for two mobs: its mount event decides, then it links.
                    game.try_mount_mob(passenger, vehicle);
                } else {
                    // An armor stand rides by naming its vehicle; the game's seat logic seats it each tick.
                    for index in 0..game.armor_stands.len() {
                        if game.armor_stands[index].id == passenger {
                            game.armor_stands[index].vehicle = Some(vehicle);
                        }
                    }
                }
            }
            RideRequest::Dismount { passenger } => {
                let current = game.mobs.iter().find(|mob| mob.id == passenger).and_then(|mob| mob.extra.mount);
                if let Some(vehicle) = current {
                    if game.native_mob_may_ride(passenger, vehicle, false) {
                        if let Some(mob) = game.mobs.iter_mut().find(|mob| mob.id == passenger) {
                            sunrise_client_lib::mobs::riders::stop_riding(mob, None);
                        }
                    }
                } else {
                    for index in 0..game.armor_stands.len() {
                        if game.armor_stands[index].id == passenger {
                            game.armor_stands[index].vehicle = None;
                        }
                    }
                }
            }
        }
    }
    let mut pairs: Vec<(u32, u32)> = Vec::new();
    for mob in &game.mobs {
        if let Some(vehicle) = mob.extra.mount {
            pairs.push((mob.id, vehicle));
        }
    }
    for index in 0..game.armor_stands.len() {
        let stand = &game.armor_stands[index];
        if let Some(vehicle) = stand.vehicle {
            pairs.push((stand.id, vehicle));
        }
    }
    for (id, display) in &game.text_displays {
        if let Some(vehicle) = display.vehicle {
            pairs.push((*id as u32, vehicle));
        }
    }
    for (id, interaction) in &game.interactions {
        if let Some(vehicle) = interaction.vehicle {
            pairs.push((*id as u32, vehicle));
        }
    }
    let local_player = game.remote.as_ref().map(|remote| remote.entity_id as u32);
    if let (Some(player), Some(vehicle)) = (local_player, game.riding) {
        pairs.push((player, vehicle));
    }
    if let Ok(mut snapshot) = SNAPSHOT.lock() {
        snapshot.dimension = dimension.to_string();
        snapshot.local_player = local_player;
        snapshot.pairs = pairs;
    }
}
