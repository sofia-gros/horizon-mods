// Entity push transport for the entity reach: consumers queue a knockback, a
// push, a leave-ground lift, or a spin attack by entity id, and the game's own knockback and
// motion paths apply the queued requests at the head of its rider
// synchronisation (pluto-entity-drain-head.rs), in the level's own tick
// context. The queue carries only the numeric id and the dimension: at
// application time the game's own entity lookup resolves the id, so no
// consumer bookkeeping can go stale.
//
// A mob's knockback goes through Mob::knockback, the game's full path: the
// native knockback hook runs on the raw strength, the mob's knockback
// resistance scales it, and the velocity is halved against the push and
// lifted by the strength exactly as LivingEntity.knockback does, with a
// sourceless damage source, as the original's bare knockback call carries
// none. A mob's push adds to its velocity and marks it for sync, as
// Entity.push does. A player's knockback goes through the game's own player
// knockback, and the player's velocity is marked so their client is told the
// new motion (the ClientboundSetEntityMotionPacket of the original). A
// guest's requests run inside the game's guest motion path, which sends
// their motion at once. The leave-ground lift is Entity.move(MoverType.SELF,
// (0, 1.3, 0)) at the head of a launch: the server's copy of the player rises
// by 1.3, or to the first ceiling the game's collision boxes name. A
// player's spin attack sets the trident spin fields (`Player.startAutoSpinAttack(20)`: 20
// ticks at 8 damage, the spin the game ends on the first hit), as a drill grapple's pull
// and a drill sling's leap start it. Only a player spins: a mob's spin request is dropped,
// and a guest's sets the guest player's own fields.
//
// Acceptance is a request taken, not a guarantee: a request whose target the
// game no longer holds, or that is not a living entity or a player, is
// dropped at application. Requests for another dimension wait for that
// dimension's synchronisation.

use std::sync::Mutex;

use super::commands::Target;
use super::Game;
use horizon_core::geometry::Vec3;

#[derive(Clone, Copy)]
enum PushRequest {
    Knockback { entity: u32, strength: f64, x: f64, z: f64 },
    Push { entity: u32, delta: [f64; 3] },
    LeaveGround { entity: u32 },
    SpinAttack { entity: u32 },
}

static QUEUE: Mutex<Vec<(String, PushRequest)>> = Mutex::new(Vec::new());

fn queue(dimension: &str, request: PushRequest) -> bool {
    let Ok(mut queue) = QUEUE.lock() else { return false };
    if queue.len() >= 1024 {
        return false;
    }
    queue.push((dimension.to_string(), request));
    true
}

/// Queues a knockback of the entity `entity` in `dimension` with `strength`
/// along the horizontal direction (`x`, `z`), applied as LivingEntity.knockback
/// applies it (see the module header); whether the request was taken. A
/// strength that is not positive and finite, or a non-finite direction, is
/// refused here.
pub fn request_knockback(dimension: &str, entity: u32, strength: f64, x: f64, z: f64) -> bool {
    if !strength.is_finite() || strength <= 0.0 || !x.is_finite() || !z.is_finite() {
        return false;
    }
    queue(dimension, PushRequest::Knockback { entity, strength, x, z })
}

/// Queues a push of the entity `entity` in `dimension`: `delta` added to its
/// velocity, as Entity.push adds it (see the module header); whether the
/// request was taken. A non-finite delta is refused here.
pub fn request_push(dimension: &str, entity: u32, delta: [f64; 3]) -> bool {
    if !delta.iter().all(|component| component.is_finite()) {
        return false;
    }
    queue(dimension, PushRequest::Push { entity, delta })
}

/// Queues a leave-ground lift of the player `entity` in `dimension` (see the
/// module header); whether the request was taken.
pub fn request_leave_ground(dimension: &str, entity: u32) -> bool {
    queue(dimension, PushRequest::LeaveGround { entity })
}

/// Queues a spin attack of the player `entity` in `dimension` (see the
/// module header); whether the request was taken.
pub fn request_spin_attack(dimension: &str, entity: u32) -> bool {
    queue(dimension, PushRequest::SpinAttack { entity })
}

/// Applies this game's queued pushes, in request order. Requests for other
/// dimensions stay queued for their own synchronisation.
pub fn drain(game: &mut Game<'_>) {
    let dimension = game.dim.id();
    let mut pending: Vec<PushRequest> = Vec::new();
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
    for request in pending {
        apply(game, request);
    }
}

fn apply(game: &mut Game<'_>, request: PushRequest) {
    let entity = match request {
        PushRequest::Knockback { entity, .. } | PushRequest::Push { entity, .. } | PushRequest::LeaveGround { entity } | PushRequest::SpinAttack { entity } => entity,
    };
    let Some(target) = game.level_entity(entity as i32) else { return };
    match target {
        Target::Mob(id) => apply_to_mob(game, id, request),
        Target::Player => apply_to_player(game, request),
        Target::Guest(conn) => {
            let Some(mut host) = game.lan.take() else { return };
            game.guest_hit_by(&mut host, conn, |guest| match request {
                PushRequest::Knockback { strength, x, z, .. } => {
                    guest.knock_back_player(strength, x, z, crate::health::HurtSource { damage_type: "minecraft:generic", causing: 0, direct: 0 });
                }
                PushRequest::Push { delta, .. } => {
                    if let Some(player) = guest.player.as_mut() {
                        player.velocity = player.velocity.add_vector(Vec3::new(delta[0], delta[1], delta[2]));
                    }
                }
                PushRequest::LeaveGround { .. } => lift_player(guest),
                PushRequest::SpinAttack { .. } => {
                    if let Some(player) = guest.player.as_mut() {
                        player.auto_spin_attack_ticks = 20;
                        player.auto_spin_attack_dmg = 8.0;
                    }
                }
            });
            game.lan = Some(host);
        }
        _ => {}
    }
}

fn apply_to_mob(game: &mut Game<'_>, id: u32, request: PushRequest) {
    let Some(mob) = game.mobs.iter_mut().find(|mob| mob.id == id) else { return };
    match request {
        PushRequest::Knockback { strength, x, z, .. } => {
            let source = crate::mobs::Damage::of(crate::mobs::DamageKind::Generic);
            mob.knockback(strength, x, z, &source);
        }
        PushRequest::Push { delta, .. } => {
            mob.velocity = mob.velocity.add_vector(Vec3::new(delta[0], delta[1], delta[2]));
            mob.needs_sync = true;
        }
        PushRequest::LeaveGround { .. } => {}
        PushRequest::SpinAttack { .. } => {}
    }
}

fn apply_to_player(game: &mut Game<'_>, request: PushRequest) {
    match request {
        PushRequest::Knockback { strength, x, z, .. } => {
            let before = game.player.as_ref().map(|player| player.velocity);
            game.knock_back_player(strength, x, z, crate::health::HurtSource { damage_type: "minecraft:generic", causing: 0, direct: 0 });
            // A server player's client is told the new motion, as the game's
            // own player knockback tells it; only when the motion changed.
            if let Some(player) = game.player.as_mut() {
                if before != Some(player.velocity) {
                    player.health.sync_velocity = true;
                }
            }
        }
        PushRequest::Push { delta, .. } => {
            if let Some(player) = game.player.as_mut() {
                player.velocity = player.velocity.add_vector(Vec3::new(delta[0], delta[1], delta[2]));
                player.health.sync_velocity = true;
            }
        }
        PushRequest::LeaveGround { .. } => lift_player(game),
        PushRequest::SpinAttack { .. } => {
            // Player.startAutoSpinAttack(20, 8, stack), as the trident's riptide sets it.
            if let Some(player) = game.player.as_mut() {
                player.auto_spin_attack_ticks = 20;
                player.auto_spin_attack_dmg = 8.0;
            }
        }
    }
}

// The launch hop: Entity.move(MoverType.SELF, (0, 1.3, 0)) for a player
// standing on the ground. The move's vertical travel stops at the first
// ceiling, so the server's copy of the player rises by 1.3 or to the lowest
// collision box bottom above them, whichever is less; the launch itself is
// the velocity queued with it, and the player's client reconciles to the
// motion as with any server move.
fn lift_player(game: &mut Game<'_>) {
    let Some(player) = game.player.as_ref() else { return };
    if !player.ground.on_ground {
        return;
    }
    let half = f64::from(player.dimensions.width) / 2.0;
    let height = f64::from(player.dimensions.height);
    let aabb = [
        player.position.x - half,
        player.position.y,
        player.position.z - half,
        player.position.x + half,
        player.position.y + height,
        player.position.z + half,
    ];
    let mut lift: f64 = 1.3;
    for block in game.native_collision_boxes(game.dim.id(), aabb) {
        let overlaps = block[0] < aabb[3] && block[3] > aabb[0] && block[2] < aabb[5] && block[5] > aabb[2];
        if overlaps && block[1] >= aabb[4] {
            lift = lift.min(block[1] - aabb[4]);
        }
    }
    if lift > 0.0 {
        if let Some(player) = game.player.as_mut() {
            player.position = Vec3::new(player.position.x, player.position.y + lift, player.position.z);
        }
    }
}
