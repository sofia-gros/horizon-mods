// The facts of a piercing projectile that wait between its impact telling and the tick that settles it: the game's own
// ignore list lives on the arrow (`pierced`, in the game's arrows module), and the impact telling is a native hook the game
// dispatches outside the projectile tick, so the two halves of one hit — the snapshot the game-side head takes when
// the hit is told, and the entities a callback says the projectile has passed through — wait here, in plain data,
// until the API side reads one and the flight pass applies the other. A telling nobody listens to costs nothing:
// the head that fills this only runs when the game dispatches an impact.
//
// The killed list (`piercedAndKilledEntities` in the original) has no home on the game's arrow at all, so it lives
// here outright, one list per projectile, kept across the arrow's hits: the API side adds the entity a piercing
// arrow's hit killed as the hit settles, the head reads the list back into the next hit's snapshot, and the list
// is forgotten when the projectile is discarded or a new projectile is born under the same id.
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

/// What the game reports for a projectile when its hit is told: the entities it has already gone through, the
/// entities it has pierced and killed so far, and the punch the shot was fired with (the shooter's bow bonus,
/// which the game records on the projectile itself, so a shot whose save names no weapon — a skeleton's — still
/// reports the punch its bow contributed).
pub struct Snapshot {
    pub pierced: Vec<u32>,
    pub killed: Vec<u32>,
    pub shot_punch: i32,
}

/// The per-hit snapshots, one entry per projectile, taken by the API side as it builds the hit.
static SNAPSHOTS: LazyLock<Mutex<HashMap<u32, Snapshot>>> = LazyLock::new(|| Mutex::new(HashMap::new()));

/// The entities a projectile has passed through, by id, as a callback recorded them — one entry per projectile,
// taken by the flight pass as the tick settles.
static PENDING: LazyLock<Mutex<HashMap<u32, Vec<u32>>>> = LazyLock::new(|| Mutex::new(HashMap::new()));

/// The entities each projectile has pierced and killed, by id, in the order it killed them: the list the original
/// keeps on the arrow itself. Kept across the arrow's whole flight; a projectile's entry is born empty, grows by
/// the kills its impacts settle, and is forgotten when the projectile ends.
static KILLED: LazyLock<Mutex<HashMap<u32, Vec<u32>>>> = LazyLock::new(|| Mutex::new(HashMap::new()));

/// Stash what the game reports for `projectile` when its hit is told, for the API side to read as it
/// builds the hit. A projectile that is told again overwrites its own entry.
pub fn stash(projectile: u32, snapshot: Snapshot) {
    let mut snapshots = SNAPSHOTS.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    if snapshots.len() > 128 && !snapshots.contains_key(&projectile) {
        snapshots.clear();
    }
    snapshots.insert(projectile, snapshot);
}

/// Take the stashed snapshot for `projectile`, if the game-side head left one.
pub fn take(projectile: u32) -> Option<Snapshot> {
    SNAPSHOTS.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).remove(&projectile)
}

/// Record the entities `projectile` has passed through, for the flight pass to add to the game's own ignore list as
/// the tick settles. A second record for the same projectile replaces the first: the API side drains a hit once, so
/// what it records is the whole list that hit settled.
pub fn record(projectile: u32, entities: Vec<u32>) {
    PENDING.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).insert(projectile, entities);
}

/// Take every pending ignore list, for the flight pass to apply.
pub fn take_all() -> Vec<(u32, Vec<u32>)> {
    PENDING.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).drain().collect()
}

/// The entities `projectile` has pierced and killed so far, for the game-side head to read into the next
/// hit's snapshot. Empty for a projectile that has killed nothing it pierced.
pub fn killed(projectile: u32) -> Vec<u32> {
    KILLED.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).get(&projectile).cloned().unwrap_or_default()
}

/// Add `entity` to the list of what `projectile` has pierced and killed: a kill the API side settled for one of
/// its hits. An entity already on the list is not added twice.
pub fn record_killed(projectile: u32, entity: u32) {
    let mut killed = KILLED.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    if killed.len() > 128 && !killed.contains_key(&projectile) {
        killed.clear();
    }
    let list = killed.entry(projectile).or_default();
    if !list.contains(&entity) {
        list.push(entity);
    }
}

/// Forget what `projectile` has pierced and killed: the projectile ended (it was discarded), or a new
/// projectile was born under an id a dead one left behind, whose kills are not the new one's.
pub fn forget_killed(projectile: u32) {
    KILLED.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).remove(&projectile);
}
