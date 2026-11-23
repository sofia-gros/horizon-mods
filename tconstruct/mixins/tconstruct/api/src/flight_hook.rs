//! The in-flight half of the tool modifiers (SlimeKnights, MIT; see LICENSE): the modules decoded in
//! `projectile_modules` (pierce, bounce, gravity, fuse, explosion, attraction, glow, angle) and the
//! flight of a thrown tool and a thrown shuriken, bound on PlutoLib's projectile tellings.
//!
//! How a flight is followed: a profile is built from the stack that was fired (its saved modifiers, at
//! the levels it carries — see `tool_stack::flight_profile`) and kept here under the projectile's
//! entity id. A bow or crossbow shot binds at `ProjectileEvents::SHOOT_BOW`: the ammunition's profile
//! is the one the `AMMO` pass just read for that player, merged with the weapon's own (the original
//! runs the shot hooks over the weapon with the fired projectile). A thrown tool and a shuriken bind
//! directly, under the id their spawn answered. The spawn pass sets a shot up from its own stack
//! (pierce, no gravity, the angle fan) whether or not a profile is bound yet.
//!
//! The original's scheduled projectile tasks (gravity's return, the fuse) run here as age checks on
//! the flight telling, which reports every projectile on every tick. Attraction on a hit is the one
//! piece that cannot act inside the impact pass (it moves entities that are not parties of the hit),
//! so a hit queues it and the next flight telling in that dimension applies it — a tick late at most
//! in any world where something flies, and the pull itself is a flight action.
//!
//! Reductions against the original, by construction: a thrown tool flies as a game arrow (its body is
//! the arrow kind, the only summonable body with a trident's physics and a native stick), so it renders
//! as an arrow, and where it lands without loyalty it drops as an item where it stuck rather than
//! resting as a trident to be picked up; loyalty's return hands the tool back as a drop at the owner's
//! feet on arrival rather than into the original slot. Weapon-side pierce cannot be set after the
//! arrow has spawned (the spawn pass reads the ammunition's own stack), so impaling on a bow alone
//! does not pierce; the ammunition's own pierce does. A thrown tool's block hit sticks where it landed
//! instead of breaking a soft block it could harvest. The glow module's wear on the launching tool is
//! not applied (the launch pass knows the tool only as a stack in a hand it cannot edit here).
use crate::{break_speed_hook::with_held_tool, projectile_modules::clamp_to, slime_entities, tool_stack};
use pluto_lib::{
    ActionTarget, EntityAction, EntityReach, ExplosionInteraction, FlightAction, ItemSlot, KnockbackDirection,
    ProjectileEvents, ProjectileFlight, ProjectileImpact, ProjectileSpawned, StackData,
};
use std::collections::HashMap;
use std::sync::Mutex;

/// A projectile being followed: its profile and the state its flight accumulates.
#[derive(Default)]
pub(crate) struct InFlight {
    profile: tool_stack::FlightProfile,
    /// Bounces used per modifier id (ProjectileBounceModule's persistent count).
    bounces_used: HashMap<String, i32>,
    gravity_restored: bool,
    fuse_done: bool,
    thrown: Option<ThrownTool>,
    shuriken: Option<ThrownShuriken>,
}

/// A thrown tool in flight (ThrownTool): the stack it carries and what its flight needs.
struct ThrownTool {
    stack: StackData,
    /// The velocity stat the tool was thrown with: its melee damage multiplier.
    multiplier: f32,
    /// Whether it has hit an entity (the original's dealtDamage: starts the loyalty return).
    dealt: bool,
    /// Whether the loyalty return has begun (the tool is worn by 1 when it leaves the ground).
    returning: bool,
}

/// A thrown shuriken in flight (ThrownShuriken): the stack it drops and its hit numbers.
struct ThrownShuriken {
    stack: StackData,
    power: f32,
    knockback: f32,
}

/// The flights being followed, by dimension and the projectile's entity id.
static FLIGHTS: Mutex<Option<HashMap<(String, u32), InFlight>>> = Mutex::new(None);

/// Attractions queued by hits, per dimension: (centre, radius, strength).
static PENDING_ATTRACTION: Mutex<Option<HashMap<String, Vec<([f64; 3], f32, f32)>>>> = Mutex::new(None);

/// The ammunition profile the AMMO pass last read, per shooting player: consumed by the shot pass.
static LAST_AMMO: Mutex<Option<HashMap<u128, tool_stack::FlightProfile>>> = Mutex::new(None);

fn with_flights<R>(run: impl FnOnce(&mut HashMap<(String, u32), InFlight>) -> R) -> R {
    let mut guard = FLIGHTS.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    run(guard.get_or_insert_with(HashMap::new))
}

/// Binds a profile (and its flight state) to a projectile that has just been fired.
pub(crate) fn bind(dimension: &str, projectile: u32, flight: InFlight) {
    if projectile == 0 { return; }
    with_flights(|flights| { flights.insert((dimension.to_owned(), projectile), flight); });
}

/// The thrown tool state a release binds for the arrow it is about to summon.
pub(crate) fn thrown_flight(profile: tool_stack::FlightProfile, stack: StackData, multiplier: f32) -> InFlight {
    InFlight { profile, thrown: Some(ThrownTool { stack, multiplier, dealt: false, returning: false }), ..Default::default() }
}

/// The shuriken state a release binds for the thrown type it is about to summon.
pub(crate) fn shuriken_flight(profile: tool_stack::FlightProfile, stack: StackData, power: f32, knockback: f32) -> InFlight {
    InFlight { profile, shuriken: Some(ThrownShuriken { stack, power, knockback }), ..Default::default() }
}

/// The profile of a stack that was fired, when it is a whole Tinkers tool.
pub(crate) fn profile_of(stack: &StackData) -> tool_stack::FlightProfile {
    with_held_tool(stack, |env, tool| tool_stack::flight_profile(&tool.custom_data, env)).unwrap_or_default()
}

fn profile_of_slot(slot: &ItemSlot) -> tool_stack::FlightProfile {
    profile_of(&StackData {
        item: slot.item.to_string(),
        count: slot.count,
        damage: slot.damage,
        max_damage: slot.max_damage,
        custom_data: slot.custom_data.clone(),
        custom_model_data: slot.custom_model_data.clone(),
        enchantments: slot.enchantments.clone().map(|map| map.into_iter().collect()).unwrap_or_default(),
    })
}

/// Merges a weapon's profile over the ammunition's, as the shot hooks run over the weapon for a shot
/// whose body is the ammunition: the weapon's shot setup wins, the lists of both apply.
fn merge(ammo: tool_stack::FlightProfile, weapon: tool_stack::FlightProfile) -> tool_stack::FlightProfile {
    tool_stack::FlightProfile {
        pierce: weapon.pierce.or(ammo.pierce),
        bounces: [ammo.bounces, weapon.bounces].concat(),
        gravity_delay: weapon.gravity_delay.or(ammo.gravity_delay),
        fuse: weapon.fuse.or(ammo.fuse),
        explosions: [ammo.explosions, weapon.explosions].concat(),
        attracts: [ammo.attracts, weapon.attracts].concat(),
        glows: [ammo.glows, weapon.glows].concat(),
        angle_directions: weapon.angle_directions.or(ammo.angle_directions),
        loyalty: weapon.loyalty.max(ammo.loyalty),
        magnet: weapon.magnet.max(ammo.magnet),
        efln: ammo.efln || weapon.efln,
        enderporting: ammo.enderporting || weapon.enderporting,
    }
}

/// The spawn pass: a shot is set up from the stack it was fired as (its own modifiers), whether or not
/// a profile is bound under its id yet.
fn on_spawn(spawn: &ProjectileSpawned) {
    let Some(stack) = &spawn.stack else { return };
    let profile = profile_of(stack);
    if profile.is_empty() { return; }
    if let Some(pierce) = profile.pierce { spawn.pierce.set(Some(pierce)); }
    if profile.gravity_delay.is_some() { spawn.no_gravity.set(Some(true)); }
    if let Some(directions) = profile.angle_directions {
        *spawn.motion_after.borrow_mut() = Some(clamp_to(directions, spawn.motion));
    }
}

/// The ammunition pass remembers the fired stack's profile for the shot pass of the same player.
fn on_ammo(request: &pluto_lib::AmmoRequest) {
    let Some(player) = request.player else { return };
    let fired = request.supplied.borrow().clone().or_else(|| request.vanilla.clone());
    let Some(fired) = fired else { return };
    let profile = profile_of_slot(&fired);
    let mut guard = LAST_AMMO.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    guard.get_or_insert_with(HashMap::new).insert(player, profile);
}

/// The shot pass binds the merged profile under the arrow's id.
fn on_shot(shot: &pluto_lib::BowShot) {
    let ammo = LAST_AMMO.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
        .get_or_insert_with(HashMap::new).remove(&shot.player).unwrap_or_default();
    let mut weapon = tool_stack::FlightProfile::default();
    for hand in [&shot.main_hand, &shot.off_hand].into_iter().flatten() {
        weapon = merge(weapon, profile_of(hand));
    }
    let profile = merge(ammo, weapon);
    if profile.is_empty() { return; }
    // The shot pass does not carry the dimension; flights are keyed with it, so the shot is bound
    // under every dimension key lazily: the flight and impact passes look their own dimension up,
    // and a shot's binding is stored under the empty key and adopted on first sight.
    with_flights(|flights| { flights.insert((String::new(), shot.projectile), InFlight { profile, ..Default::default() }); });
}

/// Takes the flight bound to a projectile in a dimension, adopting a shot's dimension-less binding.
fn take_flight(dimension: &str, projectile: u32) -> Option<InFlight> {
    with_flights(|flights| {
        if let Some(flight) = flights.remove(&(dimension.to_owned(), projectile)) { return Some(flight); }
        flights.remove(&(String::new(), projectile))
    })
}

fn put_flight(dimension: &str, projectile: u32, flight: InFlight) {
    with_flights(|flights| { flights.insert((dimension.to_owned(), projectile), flight); });
}

/// The flight pass: the scheduled tasks, the attractions a hit queued, and a thrown tool's magnet and
/// loyalty, tick by tick.
fn on_flight(flight: &ProjectileFlight) {
    // Attractions a hit queued in this dimension land on the first flight that follows.
    let pending = {
        let mut guard = PENDING_ATTRACTION.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        guard.get_or_insert_with(HashMap::new).remove(&flight.dimension).unwrap_or_default()
    };
    if !pending.is_empty() {
        let mut actions = flight.actions.borrow_mut();
        for (center, radius, strength) in pending {
            actions.push(FlightAction::PullEntities { center, radius: f64::from(radius), strength: f64::from(strength) });
        }
    }
    let Some(mut state) = take_flight(&flight.dimension, flight.projectile) else { return };
    let profile = state.profile.clone();
    // ProjectileGravityModule's scheduled task: gravity returns when the delay passes.
    if let Some(delay) = profile.gravity_delay {
        if flight.age < delay && !flight.no_gravity {
            flight.no_gravity_after.set(Some(true));
        } else if flight.age >= delay && !state.gravity_restored {
            flight.no_gravity_after.set(Some(false));
            state.gravity_restored = true;
        }
    }
    // ProjectileFuseModule's scheduled task: the fuse hooks fire, then a single-use projectile ends.
    if let Some((particle, ticks)) = &profile.fuse {
        if !state.fuse_done && flight.age >= *ticks {
            state.fuse_done = true;
            let mut actions = flight.actions.borrow_mut();
            actions.push(FlightAction::Particle { name: particle.clone(), position: flight.position, count: 10, spread: [0.0, 0.0, 0.0], speed: 0.0 });
            for (radius, fire, destroy, bonus) in &profile.explosions {
                let radius = radius + if profile.efln { *bonus } else { 0.0 };
                if radius > 0.5 {
                    actions.push(FlightAction::Explode {
                        center: flight.position,
                        radius,
                        fire: *fire,
                        interaction: if *destroy { ExplosionInteraction::Tnt } else { ExplosionInteraction::Block },
                    });
                }
            }
            for (radius, strength) in &profile.attracts {
                actions.push(FlightAction::PullEntities { center: flight.position, radius: f64::from(*radius), strength: f64::from(*strength) });
            }
            drop(actions);
            flight.discard.set(true);
            return;
        }
    }
    if let Some(thrown) = &mut state.thrown {
        thrown_tool_flight(flight, thrown, &profile);
        if flight.discard.get() { return; }
    }
    put_flight(&flight.dimension, flight.projectile, state);
}

/// A thrown tool's per-tick work: the magnet's pull on nearby items, and loyalty's return.
fn thrown_tool_flight(flight: &ProjectileFlight, thrown: &mut ThrownTool, profile: &tool_stack::FlightProfile) {
    if profile.magnet > 0 {
        flight.actions.borrow_mut().push(FlightAction::PullItems { center: flight.position, radius: 3.0, strength: 0.05 });
    }
    let stuck = flight.in_ground && flight.arrow.map(|arrow| arrow.in_ground_time > 4).unwrap_or(false);
    if profile.loyalty > 0 && (thrown.dealt || stuck) {
        if stuck && !thrown.returning {
            thrown.returning = true;
            flight.release_from_ground.set(true);
        }
        let Some(owner) = flight.owner_position else { return };
        let target = [owner[0], owner[1] + 1.5, owner[2]];
        let delta = [target[0] - flight.position[0], target[1] - flight.position[1], target[2] - flight.position[2]];
        let distance = (delta[0] * delta[0] + delta[1] * delta[1] + delta[2] * delta[2]).sqrt();
        if distance < 1.5 {
            let mut stack = thrown.stack.clone();
            if thrown.returning { stack.damage += 1; }
            flight.actions.borrow_mut().push(FlightAction::GiveStack { stack });
            flight.discard.set(true);
            return;
        }
        let speed = (flight.motion[0] * flight.motion[0] + flight.motion[1] * flight.motion[1] + flight.motion[2] * flight.motion[2]).sqrt().clamp(0.8, 2.0);
        *flight.motion_after.borrow_mut() = Some([delta[0] / distance * speed, delta[1] / distance * speed, delta[2] / distance * speed]);
        return;
    }
    // No loyalty: where the tool sticks is where it is collected from, as an item on the ground.
    if flight.in_ground && flight.arrow.map(|arrow| arrow.in_ground_time >= 2).unwrap_or(false) {
        flight.actions.borrow_mut().push(FlightAction::DropStack { stack: thrown.stack.clone(), position: flight.position });
        flight.discard.set(true);
    }
}

/// The impact pass: what lands where the projectile lands. Returns whether the game's own hit runs.
fn on_impact(hit: &ProjectileImpact) -> bool {
    let Some(mut state) = take_flight(&hit.dimension, hit.projectile) else { return true };
    let profile = state.profile.clone();
    // A shuriken's own hit replaces the game's (its damage is the tool's stat, not the type's).
    if let Some(shuriken) = &state.shuriken {
        let stack = shuriken.stack.clone();
        if let Some(_victim) = hit.entity {
            hit.actions.borrow_mut().push(EntityAction::Hurt { target: ActionTarget::Victim, amount: shuriken.power, damage_type: "minecraft:mob_projectile".into() });
            if shuriken.knockback > 0.0 {
                hit.actions.borrow_mut().push(EntityAction::Knockback { target: ActionTarget::Victim, strength: f64::from(shuriken.knockback) * 0.6, direction: KnockbackDirection::AwayFromOther });
            }
        }
        // Hit or not, the shuriken is spent and its stack drops where it landed.
        crate::throwables::drop_thrown_item(&hit.world(), hit.at, &stack.item);
        hit.discard.set(true);
        return false;
    }
    // A thrown tool's own hit: a full melee attack with the tool, then it bounces off the target.
    if hit.entity.is_some() {
        if let Some(thrown) = &mut state.thrown {
            let amount = crate::break_speed_hook::with_held_tool(&thrown.stack, |env, tool| {
                let base = (1.0 + tool_stack::stat(&tool.custom_data, crate::tool_stats::Stat::AttackDamage)) * thrown.multiplier;
                tool_stack::melee_damage(&tool.custom_data, env, base, base, &|_| false, &crate::entity_facts::Entities { attacker: None, target: None })
            }).unwrap_or(0.0);
            hit.actions.borrow_mut().push(EntityAction::Hurt { target: ActionTarget::Victim, amount, damage_type: "minecraft:player_attack".into() });
            *hit.motion_after.borrow_mut() = Some([hit.motion[0] * -0.01, hit.motion[1] * -0.1, hit.motion[2] * -0.01]);
            thrown.dealt = true;
            put_flight(&hit.dimension, hit.projectile, state);
            return false;
        }
    }
    // EnderportingModifier's projectile hit: the thrower trades places with what the shot met —
    // each move through the landing test the ender slime's teleport states — and a block hit puts
    // the thrower on the block beyond the face. The hit itself still lands. A projectile that is
    // not an arrow or a trident is spent by the move, as the original discards it; the secondary
    // arrow mark that would spare a multishot's extras is not set here (the launch telling does
    // not tell the extras apart), so every shot of an enderporting launcher swaps.
    if profile.enderporting && hit.owner >= 0 {
        let owner = hit.owner as u64;
        let world = hit.world();
        if let Some(victim) = hit.entity {
            let positions = (EntityReach::position_of(owner), EntityReach::position_of(u64::from(victim)));
            if let (Some(owner_pos), Some(victim_pos)) = positions {
                if slime_entities::teleport_lands(&world, victim_pos) && slime_entities::teleport_lands(&world, owner_pos) {
                    EntityReach::teleport(owner, victim_pos);
                    EntityReach::teleport(u64::from(victim), owner_pos);
                }
            }
        } else {
            let dest = [f64::from(hit.block[0] + hit.face[0]) + 0.5, f64::from(hit.block[1] + hit.face[1]), f64::from(hit.block[2] + hit.face[2]) + 0.5];
            if slime_entities::teleport_lands(&world, dest) && EntityReach::teleport(owner, dest)
                && !matches!(hit.kind.as_str(), "minecraft:arrow" | "minecraft:spectral_arrow" | "minecraft:trident") {
                hit.discard.set(true);
            }
        }
    }
    // ProjectileExplosionModule (priority 25, before the rest): the projectile ends in a blast.
    for (radius, fire, destroy, bonus) in &profile.explosions {
        let radius = radius + if profile.efln { *bonus } else { 0.0 };
        if radius > 0.5 {
            hit.world().explode(hit.at, radius, *fire, if *destroy { ExplosionInteraction::Tnt } else { ExplosionInteraction::Block });
            hit.discard.set(true);
            return false;
        }
    }
    // ProjectileBounceModule (priority 110): a block hit bounces while a module has bounces left.
    if hit.entity.is_none() {
        let axis = if hit.face[0] != 0 { 0 } else if hit.face[1] != 0 { 1 } else { 2 };
        for (id, limit) in &profile.bounces {
            let used = state.bounces_used.get(id).copied().unwrap_or(0);
            if used < *limit && hit.motion[axis].abs() > 0.3 {
                let mut motion = hit.motion;
                motion = [motion[0] * 0.9, motion[1] * 0.9, motion[2] * 0.9];
                motion[axis] = -motion[axis];
                *hit.motion_after.borrow_mut() = Some(motion);
                state.bounces_used.insert(id.clone(), used + 1);
                put_flight(&hit.dimension, hit.projectile, state);
                return false;
            }
        }
    }
    // ProjectileAttractMobsModule (priority 125): the pull is a flight action, queued for the next
    // flight telling in this dimension.
    if !profile.attracts.is_empty() {
        let mut guard = PENDING_ATTRACTION.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let queue = guard.get_or_insert_with(HashMap::new).entry(hit.dimension.clone()).or_default();
        for (radius, strength) in &profile.attracts { queue.push((hit.at, *radius, *strength)); }
    }
    // ProjectilePlaceGlowModule: a glow where the projectile landed.
    for (blocks, entities, _wear) in &profile.glows {
        if hit.entity.is_some() && *entities || hit.entity.is_none() && *blocks {
            let (pos, facing) = crate::throwables::glow_target(hit);
            crate::throwables::place_glow(&hit.world(), pos, facing);
            if hit.entity.is_none() { hit.discard.set(true); }
        }
    }
    put_flight(&hit.dimension, hit.projectile, state);
    true
}

/// Registers the callbacks. Called once from the product's hooks.
pub(crate) fn register() {
    ProjectileEvents::SPAWN.register(on_spawn);
    ProjectileEvents::FLIGHT.register(on_flight);
    ProjectileEvents::IMPACT.register(on_impact);
    ProjectileEvents::AMMO.register(on_ammo);
    ProjectileEvents::SHOOT_BOW.register(on_shot);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profiles_merge_with_the_weapon_winning_the_shot_setup() {
        let ammo = tool_stack::FlightProfile { pierce: Some(1), bounces: vec![("tconstruct:bounce".into(), 1)], ..Default::default() };
        let weapon = tool_stack::FlightProfile { pierce: Some(3), gravity_delay: Some(20), ..Default::default() };
        let merged = merge(ammo, weapon);
        assert_eq!(merged.pierce, Some(3));
        assert_eq!(merged.gravity_delay, Some(20));
        assert_eq!(merged.bounces.len(), 1);
    }
}
