// The ignore list a piercing projectile carries between its impact telling and the tick that settles it: the game's own
// list lives on the arrow (`pierced`, in the game's arrows module), and the impact telling is a native hook the game
// dispatches outside the projectile tick, so the two halves of one hit — the snapshot the game-side head takes when
// the hit is told, and the entities a callback says the projectile has passed through — wait here, in plain data,
// until the API side reads one and the flight pass applies the other. A telling nobody listens to costs nothing:
// the head that fills this only runs when the game dispatches an impact.
use std::collections::HashMap;
use std::sync::Mutex;

/// The entities a projectile has already gone through, by id, as the game reported them when the hit was told —
// one entry per projectile, taken by the API side as it builds the hit.
static SNAPSHOTS: Mutex<HashMap<u32, Vec<u32>>> = Mutex::new(HashMap::new());

/// The entities a projectile has passed through, by id, as a callback recorded them — one entry per projectile,
// taken by the flight pass as the tick settles.
static PENDING: Mutex<HashMap<u32, Vec<u32>>> = Mutex::new(HashMap::new());

/// Stash the ignore list the game reports for `projectile` when its hit is told, for the API side to read as it
/// builds the hit. A projectile that is told again overwrites its own entry.
pub fn stash(projectile: u32, pierced: Vec<u32>) {
    let mut snapshots = SNAPSHOTS.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    if snapshots.len() > 128 && !snapshots.contains_key(&projectile) {
        snapshots.clear();
    }
    snapshots.insert(projectile, pierced);
}

/// Take the stashed ignore list for `projectile`, if the game-side head left one.
pub fn take(projectile: u32) -> Option<Vec<u32>> {
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
