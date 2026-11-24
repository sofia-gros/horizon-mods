// The climb state a mob's tick settles, for the tick reports: a mob's own
// tick records its climbing flag and its fall-flying state here when the
// tick ran (pluto-entity-climb-record.rs, a RETURN injection on the mob
// tick), and the transport reads them when it builds the mob's tick report,
// on the same tick. The game's own `Mob::on_climbable` reads the same two
// fields: a climbing kind's flag instead of the block, and the fall-flying
// state for the glide-through exception. An entity with no record reads
// false for both: it never ticked as a mob (an armor stand, or a mob whose
// tick the injection did not see), which is the report's old answer for it.
// Entries are forgotten when the entity leaves the level; the map is
// cleared whole if it ever grows past the cap, and the next tick records
// every mob again.
use std::collections::HashMap;
use std::sync::Mutex;

use crate::mobs::Mob;

const CAP: usize = 4096;

static CLIMB: Mutex<HashMap<u32, (bool, bool)>> = Mutex::new(HashMap::new());

// Records the climb state the mob's tick settled: its climbing flag (a
// spider or cave spider pressed against a wall) and whether it glides.
pub fn record(m: &Mob) {
    let Ok(mut climb) = CLIMB.lock() else { return };
    if climb.len() >= CAP {
        climb.clear();
    }
    climb.insert(m.id, (m.climbing, m.extra.fall_flying));
}

// The climb state of the entity `entity` as of its last recorded tick: its
// climbing flag and whether it glides, or false for both when nothing was
// recorded for it.
pub fn state(entity: u32) -> (bool, bool) {
    let Ok(climb) = CLIMB.lock() else { return (false, false) };
    climb.get(&entity).copied().unwrap_or((false, false))
}

// Forgets the climb state of the entity `entity`, when it leaves the level.
pub fn forget(entity: u32) {
    let Ok(mut climb) = CLIMB.lock() else { return };
    climb.remove(&entity);
}
