// The game's half of the projectile spawn and flight tellings (pluto-projectile-flight-access): the reaches that own
// their places (a head on `add_projectile`, a return on `tick_projectiles`) call in here, the facts are built from the
// projectile itself, and what the API side settles is done to the projectile and the level as the tick settles. A
// telling nobody listens to costs one flag check.
use crate::entry::pluto_projectile_flight_access as access;
use crate::mobs::Who;
use crate::projectiles::{Projectile, ProjectileKind};
use horizon_core::geometry::Vec3;

fn arrow_facts(p: &Projectile) -> Option<access::ArrowFacts> {
    p.arrow.as_ref().map(|a| access::ArrowFacts {
        pierce: a.pierce,
        in_ground: a.in_ground,
        in_ground_time: a.in_ground_time.max(0) as u32,
        dealt_damage: a.dealt_damage,
    })
}

/// The projectile's own stack: an arrow's pickup stack, a thrown type's item.
fn stack_of(p: &Projectile) -> Option<crate::items::ItemStack> {
    if let Some(a) = &p.arrow {
        return Some(a.pickup_item.clone());
    }
    match &p.kind {
        ProjectileKind::Thrown(t) => Some(crate::items::ItemStack::new(t.item, 1)),
        _ => None,
    }
}

/// The owner as the impact telling names it: a mob's id, a player's entity id (a guest's from their connection, the
/// host's own), -1 for no owner.
fn owner_id(game: &crate::game::Game<'_>, p: &Projectile) -> i64 {
    match p.owner {
        Some(Who::Mob(id)) => i64::from(id),
        Some(Who::Player) => match p.guest {
            Some(conn) => game
                .lan_guest_entities()
                .into_iter()
                .find(|&(c, _)| c == conn)
                .map_or(0, |(_, id)| i64::from(id)),
            None => game.player.as_ref().map_or(0, |player| i64::from(player.entity_id)),
        },
        None => -1,
    }
}

fn owner_position(game: &crate::game::Game<'_>, p: &Projectile) -> Option<[f64; 3]> {
    match p.owner {
        Some(Who::Mob(id)) => game
            .mobs
            .iter()
            .find(|m| m.id == id)
            .map(|m| [m.position.x, m.position.y, m.position.z]),
        Some(Who::Player) => match p.guest {
            Some(conn) => {
                let id = game.lan_guest_entities().into_iter().find(|&(c, _)| c == conn).map(|(_, id)| id)?;
                game.mobs
                    .iter()
                    .find(|m| m.id == id)
                    .map(|m| [m.position.x, m.position.y, m.position.z])
            }
            None => game.player.as_ref().map(|player| [player.position.x, player.position.y, player.position.z]),
        },
        None => None,
    }
}

/// A projectile joining the flying list, before its first tick: the spawn pass (the head reach on `add_projectile`).
pub(crate) fn on_spawn(game: &mut crate::game::Game<'_>, p: &mut Projectile) {
    // A newborn projectile starts with no kills of its own: entity ids are reused, and the killed list
    // PlutoLib keeps per projectile (pluto-projectile-impact-access) must not hand a dead arrow's kills
    // to the one born under its id.
    crate::entry::pluto_projectile_impact_access::forget_killed(p.id);
    if !access::hears_spawn() {
        return;
    }
    let facts = access::SpawnFacts {
        dimension: game.dim.id().to_owned(),
        projectile: p.id,
        kind: p.entity_type().to_owned(),
        owner: owner_id(game, p),
        position: [p.position.x, p.position.y, p.position.z],
        motion: [p.velocity.x, p.velocity.y, p.velocity.z],
        arrow: arrow_facts(p),
        stack: stack_of(p),
    };
    let Some(outcome) = access::spawn(&facts) else { return };
    if let Some(motion) = outcome.motion {
        p.velocity = Vec3::new(motion[0], motion[1], motion[2]);
    }
    if let Some(no_gravity) = outcome.no_gravity {
        p.no_gravity = no_gravity;
    }
    if let Some(a) = &mut p.arrow {
        if let Some(pierce) = outcome.pierce {
            a.pierce = pierce;
        }
        // The pickup status the spawn pass settled (`AbstractArrow.Pickup`, read back from the
        // byte the API side sent; out of range is disallowed, as the host reads it).
        if let Some(pickup) = outcome.pickup {
            a.pickup = crate::arrows::Pickup::from_byte(i64::from(pickup));
        }
        if let Some(stack) = outcome.pickup_stack {
            a.pickup_item = stack;
        }
    }
}

/// The ignore lists the impact telling recorded, applied to the live arrows before they fly again: an entity a
/// callback said a piercing arrow passed through is added to the arrow's own list, so the game's next tick skips it
/// (its `canHitEntity` check, which passes over an entity already on the list). Only a piercing arrow takes one —
/// the game never puts a victim on a plain arrow's list — and one already there is not added twice. The host player
/// rides as `Who::Player`, every other entity as the mob id the impact telling names it by.
fn apply_ignored(game: &crate::game::Game<'_>, projectiles: &mut [Projectile], pending: Vec<(u32, Vec<u32>)>) {
    if pending.is_empty() {
        return;
    }
    let player = game.player.as_ref().and_then(|player| u32::try_from(player.entity_id).ok());
    for (projectile, entities) in pending {
        let Some(p) = projectiles.iter_mut().find(|p| p.id == projectile) else { continue };
        let Some(a) = &mut p.arrow else { continue };
        if a.pierce == 0 {
            continue;
        }
        for id in entities {
            let who = if Some(id) == player { Who::Player } else { Who::Mob(id) };
            if !a.pierced.contains(&who) {
                a.pierced.push(who);
            }
        }
    }
}

/// The end of a tick of the flying list (the return reach on `tick_projectiles`, which has put the projectiles it
/// keeps back): the flight pass. Each projectile the game still flies is told, what a callback settled is done to it,
/// and the actions it queued are done in the level in the order queued.
pub(crate) fn after_tick(game: &mut crate::game::Game<'_>) {
    // The ignore lists the impact telling recorded land here too, whether or not anyone listens to flights: a
    // piercing arrow a callback re-dealt has its victim added before it flies again, or it would meet the same
    // entity on its next tick (the game's own settle re-flies the arrow as it was before the tick, without the
    // victim). A record for a projectile that no longer flies is dropped with it.
    let pending_ignored = crate::entry::pluto_projectile_impact_access::take_all();
    if game.creatures.projectiles.is_empty() || (pending_ignored.is_empty() && !access::hears_flight()) {
        return;
    }
    let dimension = game.dim.id().to_owned();
    let mut projectiles: Vec<Projectile> = std::mem::take(&mut game.creatures.projectiles).into();
    apply_ignored(game, &mut projectiles, pending_ignored);
    let mut pending: Vec<(Vec<access::FlightAction>, Option<[f64; 3]>, [f64; 3])> = Vec::new();
    projectiles.retain_mut(|p| {
        let facts = access::FlightFacts {
            dimension: dimension.clone(),
            projectile: p.id,
            kind: p.entity_type().to_owned(),
            owner: owner_id(game, p),
            owner_position: owner_position(game, p),
            age: p.age,
            position: [p.position.x, p.position.y, p.position.z],
            motion: [p.velocity.x, p.velocity.y, p.velocity.z],
            in_ground: p.arrow.as_ref().is_some_and(|a| a.in_ground),
            no_gravity: p.no_gravity,
            arrow: arrow_facts(p),
            stack: stack_of(p),
        };
        let Some(outcome) = access::flight(&facts) else { return true };
        if !outcome.actions.is_empty() {
            pending.push((outcome.actions, facts.owner_position, facts.position));
        }
        if outcome.discard {
            return false;
        }
        if let Some(motion) = outcome.motion {
            p.velocity = Vec3::new(motion[0], motion[1], motion[2]);
        }
        if let Some(no_gravity) = outcome.no_gravity {
            p.no_gravity = no_gravity;
        }
        if let Some(pickup) = outcome.pickup {
            if let Some(a) = &mut p.arrow {
                a.pickup = crate::arrows::Pickup::from_byte(i64::from(pickup));
            }
        }
        if outcome.release_from_ground {
            if let Some(a) = &mut p.arrow {
                a.in_ground = false;
                a.in_ground_time = 0;
            }
        }
        true
    });
    game.creatures.projectiles = projectiles.into();
    for (actions, owner_at, projectile_at) in pending {
        for action in actions {
            match action {
                access::FlightAction::Explode { center, radius, fire, blast } => {
                    let blast = match blast {
                        1 => crate::game::native_explosions::NativeBlast::Block,
                        2 => crate::game::native_explosions::NativeBlast::Tnt,
                        _ => crate::game::native_explosions::NativeBlast::Keep,
                    };
                    game.native_explode(&dimension, center, radius, fire, blast);
                }
                access::FlightAction::Particle { name, position, count, spread, speed } => {
                    if let Some(ty) = horizon_client::particles::ParticleType::from_name(&name) {
                        let options = horizon_client::particles::ParticleOptions::simple(ty);
                        game.particles.send_particles(&options, position, spread, speed, count);
                        game.lan_level_particles(None, &options, position, spread, speed, count, None);
                    }
                }
                access::FlightAction::PushEntity { entity, delta } => {
                    if let Some(mob) = game.mobs.iter_mut().find(|mob| mob.id == entity) {
                        mob.velocity = mob.velocity.add_vector(Vec3::new(delta[0], delta[1], delta[2]));
                    }
                }
                access::FlightAction::PullItems { center, radius, strength } => {
                    for item in game.dropped.iter_mut() {
                        let at = [item.position.x, item.position.y, item.position.z];
                        let delta = [center[0] - at[0], center[1] - at[1], center[2] - at[2]];
                        let dist = (delta[0] * delta[0] + delta[1] * delta[1] + delta[2] * delta[2]).sqrt();
                        if dist > 0.001 && dist < radius {
                            let pull = Vec3::new(delta[0] / dist, delta[1] / dist, delta[2] / dist).scale(strength);
                            item.velocity = item.velocity.add_vector(pull);
                            item.needs_sync = true;
                        }
                    }
                }
                access::FlightAction::PullEntities { center, radius, strength } => {
                    let mut pulled = 0;
                    for mob in game.mobs.iter_mut() {
                        if pulled >= 25 {
                            break;
                        }
                        let delta = [center[0] - mob.position.x, center[1] - mob.position.y, center[2] - mob.position.z];
                        let dist = (delta[0] * delta[0] + delta[1] * delta[1] + delta[2] * delta[2]).sqrt();
                        if dist > 0.5 && dist < radius {
                            let scale = strength / dist.sqrt() / dist;
                            mob.velocity = mob.velocity.add_vector(Vec3::new(delta[0] * scale, delta[1] * scale, delta[2] * scale));
                            pulled += 1;
                        }
                    }
                }
                access::FlightAction::DropStack { stack, position } => {
                    game.spawn_item_own(stack, position, 10);
                }
                access::FlightAction::GiveStack { stack } => {
                    game.spawn_item_own(stack, owner_at.unwrap_or(projectile_at), 10);
                }
            }
        }
    }
}
