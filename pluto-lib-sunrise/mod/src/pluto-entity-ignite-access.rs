// Creeper ignite transport for the entity reach: consumers queue a creeper
// ignite by entity id, and the game's own creeper tick applies it at the
// head of its rider synchronisation (pluto-entity-drain-head.rs), in the
// level's own tick context. The queue carries only the numeric id and the
// dimension: at application time the game's own entity lookup resolves the
// id, so no consumer bookkeeping can go stale.
//
// The ignite is the creeper's own ignited flag: mob_interact sets
// `m.extra.flag` when flint and steel is used on a creeper, and
// creeper_tick reads it, starts the swell (counter 1), plays the primed
// sound and the prime_fuse game event, and explodes the creeper at max
// swell. Setting the flag here is the same ignite, applied a tick later at
// most. There is no notify call after it: the host's own ignite also calls
// native_creeper_ignite, but no mod in this tree listens to that hook (API
// 206), and the fuse itself starts from the flag alone, so the request
// carries no igniter to notify with, as the push transport's knockback
// carries no attacker.
//
// Acceptance is a request taken, not a guarantee: a request whose target the
// game no longer holds, or that is not a creeper, is dropped at
// application. Requests for another dimension wait for that dimension's
// synchronisation.

use std::sync::Mutex;

use super::commands::Target;
use super::Game;

static QUEUE: Mutex<Vec<(String, u32)>> = Mutex::new(Vec::new());

/// Queues an ignite of the creeper `entity` in `dimension` (see the module
/// header); whether the request was taken.
pub fn request_ignite_creeper(dimension: &str, entity: u32) -> bool {
    let Ok(mut queue) = QUEUE.lock() else { return false };
    if queue.len() >= 1024 {
        return false;
    }
    queue.push((dimension.to_string(), entity));
    true
}

/// Applies this game's queued creeper ignites, in request order. Requests
/// for other dimensions stay queued for their own synchronisation.
pub fn drain(game: &mut Game<'_>) {
    let dimension = game.dim.id();
    let mut pending: Vec<u32> = Vec::new();
    {
        let Ok(mut queue) = QUEUE.lock() else { return };
        let mut index = 0;
        while index < queue.len() {
            if queue[index].0 == dimension {
                pending.push(queue.remove(index).1);
            } else {
                index += 1;
            }
        }
    }
    for entity in pending {
        apply(game, entity);
    }
}

fn apply(game: &mut Game<'_>, entity: u32) {
    let Some(target) = game.level_entity(entity as i32) else { return };
    let Target::Mob(id) = target else { return };
    let Some(mob) = game.mobs.iter_mut().find(|mob| mob.id == id) else { return };
    if mob.species() != sunrise_client_lib::mobs::Species::Creeper {
        return;
    }
    mob.extra.flag = true;
}
