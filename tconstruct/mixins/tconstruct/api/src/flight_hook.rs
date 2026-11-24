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
use crate::{break_speed_hook::with_held_tool, projectile_modules::clamp_to, slime_entities, tool_damage_hook, tool_stack};
use pluto_lib::{
    ActionTarget, EntityAction, EntityReach, ExplosionInteraction, FlightAction, ItemSlot, KnockbackDirection,
    NbtCompoundExt, ProjectileEvents, ProjectileFlight, ProjectileImpact, ProjectileSpawned, StackData, StackSlot,
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
    /// Whether this shot is a multishot volley's extra arrow (the original's SECONDARY_ARROW mark):
    /// a secondary arrow never swaps its thrower (EnderportingModifier skips it) and never wears the
    /// launcher on the shot. The primary is the volley's middle arrow (see `volley_primary`).
    secondary: bool,
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

/// The volleys in progress, per shooting player: (shots seen, volley size). A crossbow with multishot
/// fires its volley as consecutive shots; the primary is the volley's middle arrow (the original's
/// `primaryIndex = ammo.getCount() / 2`, the loaded ammo's count being the volley size: 1 normally,
/// 3 with multishot). The shot telling carries no volley index or count, so the port counts the
/// consecutive shots itself, keyed by the shooter, and the count self-corrects within one volley if a
/// shot is missed (the size is recorded at the volley's first shot).
static VOLLEYS: Mutex<Option<HashMap<u128, (u8, u8)>>> = Mutex::new(None);

/// Whether the shot just fired is the volley's primary, advancing the player's volley count. A
/// volley of one is its own primary; a volley of three makes the middle shot primary.
fn volley_primary(player: u128, size: u8) -> bool {
    let size = size.max(1);
    let mut guard = VOLLEYS.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let volleys = guard.get_or_insert_with(HashMap::new);
    let (seen, known) = volleys.get(&player).copied().unwrap_or((0, size));
    let primary = seen == known / 2;
    let seen = seen + 1;
    if seen >= known { volleys.remove(&player); } else { volleys.insert(player, (seen, known)); }
    primary
}

/// The volley size of a bow stack: a crossbow fires one arrow plus two per multishot level, a
/// longbow one. The bow path fires exactly one arrow, so only a crossbow's enchantment is read.
fn volley_size(stack: &StackData) -> u8 {
    if matches!(stack.item.as_str(), "tconstruct:crossbow" | "minecraft:crossbow") {
        (1 + 2 * stack.enchantment_level("minecraft:multishot").max(0)).min(255).max(1) as u8
    } else {
        1
    }
}

/// Whether the stack is a bow or a crossbow (a Tinkers one or a vanilla one).
fn is_bow(stack: &StackData) -> bool {
    matches!(stack.item.as_str(), "tconstruct:longbow" | "tconstruct:crossbow" | "minecraft:bow" | "minecraft:crossbow")
}

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

/// A flight bound to a profile alone: a summoned projectile (a fireball) carries no thrown
/// tool or shuriken, so no magnet or loyalty runs for it — the profile's own hooks do.
pub(crate) fn profile_flight(profile: tool_stack::FlightProfile) -> InFlight {
    InFlight { profile, ..Default::default() }
}

/// Drops the flight bound to a projectile that ended: the flight pass puts the state back
/// after every tick, so a discard on its own would leave it behind.
pub(crate) fn unbind(dimension: &str, projectile: u32) {
    with_flights(|flights| { flights.remove(&(dimension.to_owned(), projectile)); });
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
        enderference: ammo.enderference || weapon.enderference,
        enderporting: ammo.enderporting || weapon.enderporting,
        tipped: weapon.tipped.or(ammo.tipped),
        spilling: [ammo.spilling, weapon.spilling].concat(),
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
    // The volley count advances on every shot, whether or not this one binds a flight: the
    // primary is a property of the volley, not of the profile. The weapon hand is the bow (the
    // main hand first); its multishot level sizes the volley, and its own enderporting — not the
    // ammunition's — decides the launch wear.
    let weapon_hand = [(&shot.main_hand, StackSlot::MainHand), (&shot.off_hand, StackSlot::OffHand)]
        .into_iter()
        .find_map(|(hand, slot)| hand.as_ref().filter(|hand| is_bow(hand)).map(|hand| (slot, hand)));
    let primary = volley_primary(shot.player, weapon_hand.as_ref().map(|(_, hand)| volley_size(hand)).unwrap_or(1));
    if primary {
        if let Some((slot, hand)) = &weapon_hand {
            if profile_of(hand).enderporting {
                // EnderportingModifier.onProjectileLaunch: the launcher is worn by 10 on a primary shot.
                if let Ok(edits) = with_held_tool(hand, |_env, tool| tool_stack::damage_edits(&tool.custom_data, 10)) {
                    if !edits.is_empty() {
                        shot.actions.borrow_mut().push(EntityAction::EditStackData { target: ActionTarget::Attacker, slot: *slot, item: hand.item.clone(), edits });
                    }
                }
            }
        }
    }
    if profile.is_empty() { return; }
    // The shot pass does not carry the dimension; flights are keyed with it, so the shot is bound
    // under every dimension key lazily: the flight and impact passes look their own dimension up,
    // and a shot's binding is stored under the empty key and adopted on first sight.
    with_flights(|flights| { flights.insert((String::new(), shot.projectile), InFlight { profile, secondary: !primary, ..Default::default() }); });
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

/// The damage type a thrown tool's hit is dealt as (TinkerDamageTypes.THROWN_TOOL): a projectile
/// type, in the projectile tag — an enderman dodges it as it dodges any shot.
const THROWN_TOOL_DAMAGE: &str = "tconstruct:thrown_tool";

/// The damage type that replaces it under the enderference override (TinkerDamageTypes.MELEE_THROWN_TOOL):
/// the same type (same death message, same scaling) outside the projectile tag, so the hit lands
/// on a mob projectiles cannot touch.
const MELEE_THROWN_TOOL_DAMAGE: &str = "tconstruct:melee_thrown_tool";

/// TinkerEffects.needsEnderferenceOverride, on the facts an impact reports: the target is an
/// enderman and it carries the enderference effect. Both halves are the original's — either one
/// alone (an enderman without the effect, any other kind carrying it) is no override. Every
/// damage-type choice below that turns on the override reads it through here.
fn needs_enderference_override(entity_kind: &str, entity_effects: &[String]) -> bool {
    entity_kind == "minecraft:enderman"
        && entity_effects.iter().any(|effect| effect.as_str() == crate::teleport_dodge::ENDERFERENCE)
}

/// Which of the two types a thrown tool's hit on this target is dealt as
/// (ToolAttackContext.makeDamageSource, through TinkerEffects.needsEnderferenceOverride): the
/// melee type only for an enderman carrying the enderference effect; every other target — an
/// enderman without the effect, any other kind carrying it — takes the thrown type.
fn thrown_tool_damage_type(entity_kind: &str, entity_effects: &[String]) -> &'static str {
    if needs_enderference_override(entity_kind, entity_effects) {
        MELEE_THROWN_TOOL_DAMAGE
    } else {
        THROWN_TOOL_DAMAGE
    }
}

/// The damage type a shuriken's hit is dealt as on any other target (DamageSources.thrown): a
/// projectile type, in the projectile tag.
const THROWN_DAMAGE: &str = "minecraft:thrown";

/// The damage type that replaces it under the enderference override (TinkerDamageTypes.MELEE_THROWN):
/// outside the projectile tag, so the hit lands on a mob projectiles cannot touch.
const MELEE_THROWN_DAMAGE: &str = "tconstruct:melee_thrown";

/// Which of the two types a shuriken's hit on this target is dealt as (ThrownShuriken.onHitEntity,
/// through TinkerEffects.needsEnderferenceOverride).
fn shuriken_damage_type(entity_kind: &str, entity_effects: &[String]) -> &'static str {
    if needs_enderference_override(entity_kind, entity_effects) {
        MELEE_THROWN_DAMAGE
    } else {
        THROWN_DAMAGE
    }
}

/// The damage type ModifierEvents deals an arrow's reimplemented hit as
/// (TinkerDamageTypes.MELEE_ARROW): outside the projectile tag, so it lands on the enderman the
/// effect exposes.
const MELEE_ARROW_DAMAGE: &str = "tconstruct:melee_arrow";

/// Whether ModifierEvents' arrow reimplementation answers this hit: an arrow (the trident is on
/// the original's ENDERFERENCE_ARROW_BLACKLIST, and thrown tools and shurikens answer their own
/// hits above) meeting a target the enderference override exposes. The reimplementation is the
/// enderference effect's machinery, not the projectile's own hook: it answers arrows no flight
/// is bound for as much as bound ones.
fn arrow_override(hit: &ProjectileImpact) -> bool {
    hit.entity.is_some()
        && matches!(hit.kind.as_str(), "minecraft:arrow" | "minecraft:spectral_arrow")
        && needs_enderference_override(&hit.entity_kind, &hit.entity_effects)
}

/// The damage of ModifierEvents' reimplemented arrow hit: the arrow's speed times its base
/// damage, rounded up (clamped at the integer maximum, as the original clamps), with a critical
/// arrow's bonus of a roll under `damage / 2 + 2` added on top. The same formula the game's own
/// arrow hit computes — the reimplementation exists because the game's hit never lands on an
/// enderman, not because its numbers differ.
fn arrow_hit_damage(motion: [f64; 3], base_damage: f64, critical: bool, roll: &mut dyn FnMut(i32) -> i32) -> f32 {
    let speed = (motion[0] * motion[0] + motion[1] * motion[1] + motion[2] * motion[2]).sqrt();
    let mut damage = (speed * base_damage).clamp(0.0, f64::from(i32::MAX)).ceil() as i32;
    if critical {
        damage = damage.saturating_add(roll(damage / 2 + 2));
    }
    damage as f32
}

/// Deals ModifierEvents' reimplemented arrow hit on the impact's target: the arrow's own damage
/// as the melee type, the target lit for 5 seconds when the arrow burns, and the arrow spent
/// unless it pierces (a piercing arrow flies on; the original's pierce bookkeeping — the ignore
/// list and the killed list — is the game's own, which a cancelled hit never runs, so a piercing
/// arrow may meet the same enderman again). The arrow's numbers are read off its saved data, as
/// the game saves them: `damage` the base damage (2.0 when the save names none, the game's own
/// default), `crit`, `PierceLevel`, and `Fire` the ticks it has left burning. The punch knockback
/// the original adds is not dealt: the arrow's save does not carry its knockback, and the game's
/// own hit — which applies it — is the hit being replaced.
fn deal_arrow_override(hit: &ProjectileImpact) {
    let base_damage = hit.data.get_double("damage", 2.0);
    let critical = hit.data.get_int("crit", 0) != 0;
    let amount = arrow_hit_damage(hit.motion, base_damage, critical, &mut |bound| {
        (f64::from(tool_damage_hook::random()) * f64::from(bound.max(1))) as i32
    });
    let mut actions = hit.actions.borrow_mut();
    actions.push(EntityAction::Hurt { target: ActionTarget::Victim, amount, damage_type: MELEE_ARROW_DAMAGE.into() });
    if hit.data.get_int("Fire", 0) > 0 {
        actions.push(EntityAction::SetFire { target: ActionTarget::Victim, seconds: 5 });
    }
    drop(actions);
    if hit.data.get_int("PierceLevel", 0) <= 0 {
        hit.discard.set(true);
    }
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
            // EnderportingModifier.onProjectileFuseFinish: the owner is moved to the expired
            // projectile. The reach cannot move a player (the host has no call for it), so a
            // player owner is queued for the next player tick (see enderporting_hook); a mob
            // owner moves at once. A fishing rod's bobber wears its rod by 10 here too
            // (updateFishingRod with the draw-speed cooldown, whose surface is the sleeves
            // cooldown — not yet merged — so the wear alone is queued, the cooldown waits).
            if profile.enderporting && !state.secondary && flight.owner >= 0 {
                let owner = flight.owner as u64;
                if !EntityReach::teleport(owner, flight.position) {
                    crate::enderporting_hook::queue_fuse_teleport(owner, &flight.dimension, flight.position);
                }
                if flight.kind == "minecraft:fishing_bobber" {
                    crate::enderporting_hook::queue_rod_wear(owner, 10);
                }
            }
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
    let Some(mut state) = take_flight(&hit.dimension, hit.projectile) else {
        // No flight is bound, so none of the projectile's own hooks can run — but ModifierEvents'
        // arrow reimplementation is not the projectile's hook, it is the enderference effect's,
        // and it answers an unbound arrow (a skeleton's, a dispenser's) as much as a bound one.
        if arrow_override(hit) {
            deal_arrow_override(hit);
            return false;
        }
        return true;
    };
    let profile = state.profile.clone();
    // ToolEvents.projectileHit's enderference gate (TinkerEffects.canHitWithProjectile, plus the
    // projectile's own mark): an entity hit runs the projectile's hit hooks only when the target can
    // be hit with a projectile — not an enderman, or an enderman carrying the enderference effect —
    // or when the projectile itself carries the mark (profile.enderference). A gated-out hit still
    // lands as the game's own hit (the original does not cancel the event, it only skips the modifier
    // hooks), so the flight is put back and the hit is allowed; a piercing arrow flies on to the next
    // entity exactly as the original's uncancelled event lets it. Whether an enderman dodges a shot
    // at all is the game's own behaviour, untouched by this gate.
    if hit.entity.is_some()
        && hit.entity_kind == "minecraft:enderman"
        && !hit.entity_effects.iter().any(|effect| effect.as_str() == crate::teleport_dodge::ENDERFERENCE)
        && !profile.enderference
    {
        put_flight(&hit.dimension, hit.projectile, state);
        return true;
    }
    // A shuriken's own hit replaces the game's (its damage is the tool's stat, not the type's).
    // Its damage source is the one ThrownShuriken.onHitEntity builds: the vanilla thrown type,
    // or the melee thrown type on an enderman the enderference effect exposes.
    if let Some(shuriken) = &state.shuriken {
        let stack = shuriken.stack.clone();
        if let Some(_victim) = hit.entity {
            let damage_type = shuriken_damage_type(&hit.entity_kind, &hit.entity_effects);
            hit.actions.borrow_mut().push(EntityAction::Hurt { target: ActionTarget::Victim, amount: shuriken.power, damage_type: damage_type.into() });
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
    // Its damage source is the one ToolAttackContext.makeDamageSource builds for a thrown tool:
    // the thrown type, or the melee type on an enderman the enderference effect exposes — the
    // port dealt every such hit as a player attack before, which no enderman ever dodged and no
    // projectile-keyed protection ever read as a shot.
    if hit.entity.is_some() {
        if let Some(thrown) = &mut state.thrown {
            let amount = crate::break_speed_hook::with_held_tool(&thrown.stack, |env, tool| {
                let base = (1.0 + tool_stack::stat(&tool.custom_data, crate::tool_stats::Stat::AttackDamage)) * thrown.multiplier;
                tool_stack::melee_damage(&tool.custom_data, env, base, base, &|_| false, &crate::entity_facts::Entities { attacker: None, target: None })
            }).unwrap_or(0.0);
            let damage_type = thrown_tool_damage_type(&hit.entity_kind, &hit.entity_effects);
            hit.actions.borrow_mut().push(EntityAction::Hurt { target: ActionTarget::Victim, amount, damage_type: damage_type.into() });
            *hit.motion_after.borrow_mut() = Some([hit.motion[0] * -0.01, hit.motion[1] * -0.1, hit.motion[2] * -0.01]);
            thrown.dealt = true;
            put_flight(&hit.dimension, hit.projectile, state);
            return false;
        }
    }
    // EnderportingModifier's projectile hit: the thrower trades places with what the shot met —
    // each move through the landing test the ender slime's teleport states — and a block hit puts
    // the thrower on the block beyond the face. The hit itself still lands. A projectile that is
    // not an arrow or a trident is spent by the move, as the original discards it. A secondary
    // arrow of a multishot volley never swaps (the launch pass marks it), and a fishing rod's
    // bobber wears its rod by 10 on a swap that lands (updateFishingRod; the draw-speed cooldown
    // waits for the sleeves surface — see the fuse branch above).
    if profile.enderporting && !state.secondary && hit.owner >= 0 {
        let owner = hit.owner as u64;
        let world = hit.world();
        if let Some(victim) = hit.entity {
            let positions = (EntityReach::position_of(owner), EntityReach::position_of(u64::from(victim)));
            if let (Some(owner_pos), Some(victim_pos)) = positions {
                if slime_entities::teleport_lands(&world, victim_pos) && slime_entities::teleport_lands(&world, owner_pos) {
                    let swapped = EntityReach::teleport(owner, victim_pos) && EntityReach::teleport(u64::from(victim), owner_pos);
                    if swapped && hit.kind == "minecraft:fishing_bobber" {
                        crate::enderporting_hook::queue_rod_wear(owner, 10);
                    }
                }
            }
        } else {
            let dest = [f64::from(hit.block[0] + hit.face[0]) + 0.5, f64::from(hit.block[1] + hit.face[1]), f64::from(hit.block[2] + hit.face[2]) + 0.5];
            if slime_entities::teleport_lands(&world, dest) && EntityReach::teleport(owner, dest) {
                if hit.kind == "minecraft:fishing_bobber" {
                    crate::enderporting_hook::queue_rod_wear(owner, 10);
                }
                if !matches!(hit.kind.as_str(), "minecraft:arrow" | "minecraft:spectral_arrow" | "minecraft:trident") {
                    hit.discard.set(true);
                }
            }
        }
    }
    // TippedModule.onProjectileHitEntity: the potion the fired stack was tipped with,
    // applied to the target at the divisor the entry's level sets. The original zeroes
    // the target's invulnerable time around the application; the impact actions land
    // after the callbacks, in order, which is the closest the telling allows.
    if hit.entity.is_some() {
        if let Some((potion, level)) = &profile.tipped {
            if let Ok(potion_id) = pluto_lib::Identifier::parse(potion) {
                for effect in crate::potion_contents::effects(&potion_id) {
                    // An instant effect: applyInstantenousEffect scales its amount by
                    // 1 / (divisor * 0.75), which an effect action cannot carry, so it
                    // goes on at its own amplifier for a tick (the game applies an
                    // instant effect as it is added) and the amount's scaling is the
                    // remainder. A lasting effect scales its duration exactly.
                    let duration = if effect.instant { 1 } else { crate::potion_contents::scaled_duration(effect.duration, *level) };
                    hit.actions.borrow_mut().push(EntityAction::AddEffect { target: ActionTarget::Victim, effect: effect.effect.to_string(), amplifier: effect.amplifier, duration });
                }
            }
        }
    }
    // SpillingModule.onLauncherHitEntity: the firing tool's tank fluid, applied to the target at
    // the module's level. The profile carries a snapshot of the tank, and what the application
    // spends is spent from the snapshot — the firing tool itself is out of this telling's reach,
    // the standing shape of the launcher applications here. The original's damageDealt > 0 gate
    // cannot be read at the impact telling, the same remainder as the tipped application above.
    if hit.entity.is_some() && !profile.spilling.is_empty() {
        if let (Some(effects), Some(tags)) = (crate::common::fluid_effects(), crate::common::smeltery_data().and_then(|data| data.read().ok().map(|data| data.tags.clone()))) {
            let effects = effects.read().expect("fluid effects lock poisoned");
            let stack = state.thrown.as_ref().map(|thrown| crate::fluid_world::stack_of_data(&thrown.stack))
                .or_else(|| state.shuriken.as_ref().map(|shuriken| crate::fluid_world::stack_of_data(&shuriken.stack)))
                .unwrap_or_else(crate::fluid_effects::Stack::empty);
            let mut world = crate::fluid_world::LiveWorld::new(hit.world());
            for (level, fluid) in &profile.spilling {
                let Some(fluid) = fluid else { continue };
                if !effects.describe(fluid, &tags).map_or(false, |(_, has_entity, _)| has_entity) { continue; }
                let mut target = crate::fluid_world::LiveTarget::victim(hit, &hit.actions);
                let mut context = crate::fluid_effects::EntityContext {
                    env: crate::fluid_effects::Env { world: &mut world, holder: crate::fluid_world::holder(stack.clone(), Some((hit.projectile as i32, hit.motion)), hit.at) },
                    target: &mut target,
                    location: hit.at,
                };
                if effects.apply_to_entity(fluid, *level, &mut context, &tags) > 0 {
                    crate::fluid_world::spawn_fluid_particles(&hit.world(), fluid, hit.at, 10);
                }
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
    // ModifierEvents' arrow reimplementation, last: an arrow that met an enderman the
    // enderference effect exposes is answered here rather than by the game's own hit, which the
    // enderman dodges as it dodges any shot — the hit hooks above have run, as the enderference
    // gate lets them on such a target, and the reimplementation deals the arrow's damage as the
    // melee type and cancels the hit so the arrow does not bounce.
    if arrow_override(hit) {
        deal_arrow_override(hit);
        put_flight(&hit.dimension, hit.projectile, state);
        return false;
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

    #[test]
    fn profiles_merge_carrying_either_side_projectile_marks() {
        // The volatile flags' marks (fins' efln, enderference's) ride the merged profile from
        // whichever stack carries them, as the shot hooks run over both.
        let ammo = tool_stack::FlightProfile { efln: true, ..Default::default() };
        let weapon = tool_stack::FlightProfile { enderference: true, ..Default::default() };
        let merged = merge(ammo, weapon);
        assert!(merged.efln);
        assert!(merged.enderference);
        assert!(!merged.is_empty(), "a lone enderference mark still binds a flight");
    }

    /// An impact for the gate tests: an attracted profile (the one hook whose firing is visible
    /// without the world) bound under the projectile, meeting the named kind with the named effects.
    fn gated_hit(dimension: &str, entity: Option<u32>, kind: &str, effects: Vec<String>, enderference: bool) -> ProjectileImpact {
        bind(dimension, 7, InFlight { profile: tool_stack::FlightProfile { attracts: vec![(3.0, 0.5)], enderference, ..Default::default() }, ..Default::default() });
        ProjectileImpact {
            dimension: dimension.into(),
            projectile: 7,
            kind: "minecraft:arrow".into(),
            owner: 1,
            entity,
            entity_kind: kind.into(),
            entity_effects: effects,
            block: [0, 64, 0],
            face: [0, 1, 0],
            at: [0.5, 64.5, 0.5],
            motion: [0.0, 0.0, -1.0],
            data: pluto_lib::NbtCompound::new(),
            discard: std::cell::Cell::new(false),
            motion_after: std::cell::RefCell::new(None),
            actions: std::cell::RefCell::new(Vec::new()),
        }
    }

    /// A thrown tool's impact for the damage-type tests: a thrown flight bound under the
    /// projectile, meeting the named kind carrying the named effects; `mark` says the tool
    /// itself carries the enderference volatile flag. The stack is no loaded tool, so the
    /// attack's amount reads 0 — the type the hit is dealt as is what is under test.
    fn thrown_hit(dimension: &str, kind: &str, effects: Vec<String>, mark: bool) -> ProjectileImpact {
        let stack = StackData {
            item: "tconstruct:hand_axe".into(),
            count: 1,
            damage: 0,
            max_damage: None,
            custom_data: pluto_lib::NbtCompound::new(),
            custom_model_data: None,
            enchantments: Default::default(),
        };
        let profile = tool_stack::FlightProfile { enderference: mark, ..Default::default() };
        bind(dimension, 7, thrown_flight(profile, stack, 1.0));
        ProjectileImpact {
            dimension: dimension.into(),
            projectile: 7,
            kind: "minecraft:arrow".into(),
            owner: 1,
            entity: Some(9),
            entity_kind: kind.into(),
            entity_effects: effects,
            block: [0, 64, 0],
            face: [0, 1, 0],
            at: [0.5, 64.5, 0.5],
            motion: [0.0, 0.0, -1.0],
            data: pluto_lib::NbtCompound::new(),
            discard: std::cell::Cell::new(false),
            motion_after: std::cell::RefCell::new(None),
            actions: std::cell::RefCell::new(Vec::new()),
        }
    }

    /// The attractions a hit queued in a dimension, drained so cases do not see each other.
    fn queued_attractions(dimension: &str) -> Vec<([f64; 3], f32, f32)> {
        let mut guard = PENDING_ATTRACTION.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        guard.get_or_insert_with(HashMap::new).remove(dimension).unwrap_or_default()
    }

    #[test]
    fn the_enderference_gate_skips_the_hit_hooks_on_a_plain_enderman() {
        // A plain enderman, a projectile without the mark: no hit hook fires, but the game's own hit
        // still runs (the original does not cancel the event, it only skips the modifier hooks).
        let dimension = "test:endergate-plain";
        let hit = gated_hit(dimension, Some(9), "minecraft:enderman", Vec::new(), false);
        assert!(on_impact(&hit), "a gated-out hit still lands as the game's own hit");
        assert!(!hit.discard.get(), "nothing consumes the projectile");
        assert!(queued_attractions(dimension).is_empty(), "no hit hook fired");
    }

    #[test]
    fn the_enderference_gate_opens_for_the_mark_the_effect_and_other_kinds() {
        // Each of the original's three openings runs the hooks: the projectile carries the mark, the
        // enderman carries the effect, or the target is no enderman at all; a block hit has no target
        // to gate on, so its hooks run too.
        for (dimension, entity, kind, effects, mark) in [
            ("test:endergate-mark", Some(9), "minecraft:enderman", Vec::new(), true),
            ("test:endergate-effect", Some(9), "minecraft:enderman", vec![crate::teleport_dodge::ENDERFERENCE.to_owned()], false),
            ("test:endergate-zombie", Some(9), "minecraft:zombie", Vec::new(), false),
            ("test:endergate-block", None, "", Vec::new(), false),
        ] {
            let hit = gated_hit(dimension, entity, kind, effects, mark);
            assert!(on_impact(&hit), "{dimension}: the hit itself always lands");
            assert_eq!(queued_attractions(dimension).len(), 1, "{dimension}: the hit hooks ran");
        }
    }
    /// The damage type of the one hurt a thrown hit dealt.
    fn dealt_type(hit: &ProjectileImpact) -> String {
        let actions = hit.actions.borrow();
        assert_eq!(actions.len(), 1, "a thrown tool's hit deals exactly its attack");
        match &actions[0] {
            EntityAction::Hurt { damage_type, .. } => damage_type.clone(),
            _ => panic!("a thrown tool's hit deals a hurt"),
        }
    }

    #[test]
    fn a_thrown_tool_hits_an_exposed_enderman_with_the_melee_type() {
        // ToolAttackContext.makeDamageSource: an enderman carrying the enderference effect is
        // the one target a thrown tool strikes with the melee type, so the hit lands on a mob
        // that dodges projectiles.
        let hit = thrown_hit("test:endermelee-exposed", "minecraft:enderman", vec![crate::teleport_dodge::ENDERFERENCE.to_owned()], false);
        assert!(!on_impact(&hit), "a thrown tool's own hit replaces the game's");
        assert_eq!(dealt_type(&hit), MELEE_THROWN_TOOL_DAMAGE);
    }

    #[test]
    fn a_thrown_tool_keeps_the_thrown_type_on_every_other_target() {
        // An enderman without the effect, and any other kind — even one carrying the effect —
        // take the thrown type: the override names the enderman alone. The plain enderman is
        // met by a marked tool: the ToolEvents gate lets a marked tool's hit hooks run on it,
        // and makeDamageSource still deals the thrown type, as the target itself lacks the
        // effect — the mark opens the hooks, it does not retitle the damage.
        for (dimension, kind, effects, mark) in [
            ("test:endermelee-plain-enderman", "minecraft:enderman", Vec::new(), true),
            ("test:endermelee-marked-zombie", "minecraft:zombie", vec![crate::teleport_dodge::ENDERFERENCE.to_owned()], false),
            ("test:endermelee-plain-zombie", "minecraft:zombie", Vec::new(), false),
        ] {
            let hit = thrown_hit(dimension, kind, effects, mark);
            assert!(!on_impact(&hit), "{dimension}: a thrown tool's own hit replaces the game's");
            assert_eq!(dealt_type(&hit), THROWN_TOOL_DAMAGE, "{dimension}");
        }
    }

    #[test]
    fn the_thrown_tool_type_turns_on_the_enderman_and_the_effect_together() {
        // The qualifying boundary of TinkerEffects.needsEnderferenceOverride, read straight off
        // the choice: the kind and the effect must both be present, and no other effect stands in.
        let enderference = crate::teleport_dodge::ENDERFERENCE;
        assert_eq!(thrown_tool_damage_type("minecraft:enderman", &[enderference.to_owned()]), MELEE_THROWN_TOOL_DAMAGE);
        assert_eq!(thrown_tool_damage_type("minecraft:enderman", &[]), THROWN_TOOL_DAMAGE);
        assert_eq!(thrown_tool_damage_type("minecraft:enderman", &["minecraft:speed".to_owned()]), THROWN_TOOL_DAMAGE);
        assert_eq!(thrown_tool_damage_type("minecraft:zombie", &[enderference.to_owned()]), THROWN_TOOL_DAMAGE);
        assert_eq!(thrown_tool_damage_type("", &[enderference.to_owned()]), THROWN_TOOL_DAMAGE);
    }

    #[test]
    fn the_shuriken_type_turns_on_the_enderman_and_the_effect_together() {
        // The same boundary, read off ThrownShuriken.onHitEntity's choice: the melee thrown type
        // only where the override holds, the vanilla thrown type everywhere else.
        let enderference = crate::teleport_dodge::ENDERFERENCE;
        assert_eq!(shuriken_damage_type("minecraft:enderman", &[enderference.to_owned()]), MELEE_THROWN_DAMAGE);
        assert_eq!(shuriken_damage_type("minecraft:enderman", &[]), THROWN_DAMAGE);
        assert_eq!(shuriken_damage_type("minecraft:enderman", &["minecraft:speed".to_owned()]), THROWN_DAMAGE);
        assert_eq!(shuriken_damage_type("minecraft:zombie", &[enderference.to_owned()]), THROWN_DAMAGE);
        assert_eq!(shuriken_damage_type("", &[enderference.to_owned()]), THROWN_DAMAGE);
    }

    #[test]
    fn the_reimplemented_arrow_damage_is_the_game_s_own_formula() {
        // Speed times base damage, rounded up; a critical arrow adds a roll under
        // damage / 2 + 2 (ModifierEvents reads the same numbers AbstractArrow.onHitEntity does).
        assert_eq!(arrow_hit_damage([0.0, 0.0, -3.0], 2.0, false, &mut |_| panic!("no roll without a crit")), 6.0);
        assert_eq!(arrow_hit_damage([1.5, 0.0, 0.0], 2.0, false, &mut |_| 0), 3.0);
        assert_eq!(arrow_hit_damage([0.0, 0.0, -3.0], 2.0, true, &mut |bound| { assert_eq!(bound, 5); 4 }), 10.0);
        assert_eq!(arrow_hit_damage([0.0, 0.0, 0.0], 2.0, false, &mut |_| 0), 0.0);
    }

    /// An arrow's impact for the reimplementation tests: the named projectile kind meeting the
    /// named kind carrying the named effects, flying at speed 3, its saved data carrying the
    /// named base damage, crit flag, pierce level and remaining fire ticks. `bound` first binds
    /// an attracting flight under the projectile (the one hook whose firing is visible without
    /// the world), as a shot with a profile would.
    fn arrow_hit(dimension: &str, kind: &str, entity_kind: &str, effects: Vec<String>, base_damage: f64, critical: bool, pierce: i8, fire: i16, bound: bool) -> ProjectileImpact {
        if bound {
            bind(dimension, 7, InFlight { profile: tool_stack::FlightProfile { attracts: vec![(3.0, 0.5)], ..Default::default() }, ..Default::default() });
        }
        let mut data = pluto_lib::NbtCompound::new();
        data.insert("damage".into(), pluto_lib::Nbt::Double(base_damage));
        data.insert("crit".into(), pluto_lib::Nbt::Byte(if critical { 1 } else { 0 }));
        data.insert("PierceLevel".into(), pluto_lib::Nbt::Byte(pierce));
        data.insert("Fire".into(), pluto_lib::Nbt::Short(fire));
        ProjectileImpact {
            dimension: dimension.into(),
            projectile: 7,
            kind: kind.into(),
            owner: 1,
            entity: Some(9),
            entity_kind: entity_kind.into(),
            entity_effects: effects,
            block: [0, 64, 0],
            face: [0, 1, 0],
            at: [0.5, 64.5, 0.5],
            motion: [0.0, 0.0, -3.0],
            data,
            discard: std::cell::Cell::new(false),
            motion_after: std::cell::RefCell::new(None),
            actions: std::cell::RefCell::new(Vec::new()),
        }
    }

    /// The hurts an impact dealt, as (damage type, amount) pairs.
    fn dealt_hurts(hit: &ProjectileImpact) -> Vec<(String, f32)> {
        hit.actions.borrow().iter().filter_map(|action| match action {
            EntityAction::Hurt { damage_type, amount, .. } => Some((damage_type.clone(), *amount)),
            _ => None,
        }).collect()
    }

    #[test]
    fn an_unbound_arrow_is_reimplemented_on_an_exposed_enderman() {
        // A skeleton's arrow, no flight bound: ModifierEvents still answers, dealing the arrow's
        // damage (speed 3 × base 2 = 6) as the melee type, and the arrow is spent.
        let hit = arrow_hit("test:enderarrow-unbound", "minecraft:arrow", "minecraft:enderman", vec![crate::teleport_dodge::ENDERFERENCE.to_owned()], 2.0, false, 0, 0, false);
        assert!(!on_impact(&hit), "the reimplementation replaces the game's hit");
        assert_eq!(dealt_hurts(&hit), vec![(MELEE_ARROW_DAMAGE.to_owned(), 6.0)]);
        assert!(hit.discard.get(), "a non-piercing arrow is spent");
    }

    #[test]
    fn a_bound_arrow_is_reimplemented_after_its_hit_hooks() {
        // A shot with a profile: the hit hooks run (the attraction queues), then the
        // reimplementation deals the damage; a burning arrow lights the target for 5 seconds.
        let dimension = "test:enderarrow-bound";
        let hit = arrow_hit(dimension, "minecraft:spectral_arrow", "minecraft:enderman", vec![crate::teleport_dodge::ENDERFERENCE.to_owned()], 2.0, false, 0, 100, true);
        assert!(!on_impact(&hit), "the reimplementation replaces the game's hit");
        assert_eq!(dealt_hurts(&hit), vec![(MELEE_ARROW_DAMAGE.to_owned(), 6.0)]);
        assert!(hit.actions.borrow().iter().any(|action| matches!(action, EntityAction::SetFire { seconds: 5, .. })), "a burning arrow lights its target");
        let mut guard = PENDING_ATTRACTION.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let queued = guard.get_or_insert_with(HashMap::new).remove(dimension).unwrap_or_default();
        assert_eq!(queued.len(), 1, "the attraction hook ran before the reimplementation");
    }

    #[test]
    fn a_piercing_arrow_flies_on_after_the_reimplemented_hit() {
        let hit = arrow_hit("test:enderarrow-pierce", "minecraft:arrow", "minecraft:enderman", vec![crate::teleport_dodge::ENDERFERENCE.to_owned()], 2.0, false, 1, 0, false);
        assert!(!on_impact(&hit), "the reimplementation replaces the game's hit");
        assert_eq!(dealt_hurts(&hit), vec![(MELEE_ARROW_DAMAGE.to_owned(), 6.0)]);
        assert!(!hit.discard.get(), "a piercing arrow is not spent");
    }

    #[test]
    fn the_reimplementation_answers_nothing_else() {
        // A plain enderman (no effect), a marked zombie, and a trident (the original's blacklist)
        // all keep the game's own hit, bound or not.
        for (dimension, kind, entity_kind, effects) in [
            ("test:enderarrow-plain-enderman", "minecraft:arrow", "minecraft:enderman", Vec::new()),
            ("test:enderarrow-marked-zombie", "minecraft:arrow", "minecraft:zombie", vec![crate::teleport_dodge::ENDERFERENCE.to_owned()]),
            ("test:enderarrow-trident", "minecraft:trident", "minecraft:enderman", vec![crate::teleport_dodge::ENDERFERENCE.to_owned()]),
        ] {
            let hit = arrow_hit(dimension, kind, entity_kind, effects, 2.0, false, 0, 0, false);
            assert!(on_impact(&hit), "{dimension}: the game's own hit runs");
            assert!(dealt_hurts(&hit).is_empty(), "{dimension}");
            assert!(!hit.discard.get(), "{dimension}");
        }
    }

    #[test]
    fn volley_primary_is_the_middle_arrow_of_three_and_the_only_arrow_of_one() {
        // A volley of three: the first and third are secondary, the middle primary.
        let player = 4242u128;
        assert!(!volley_primary(player, 3));
        assert!(volley_primary(player, 3));
        assert!(!volley_primary(player, 3));
        // The volley is spent: the next shot starts a new one.
        assert!(volley_primary(player, 1));
    }

    #[test]
    fn volley_size_is_three_for_a_multishot_crossbow_and_one_otherwise() {
        let crossbow = StackData { item: "tconstruct:crossbow".into(), enchantments: [("minecraft:multishot".to_string(), 1)].into_iter().collect(), ..Default::default() };
        assert_eq!(volley_size(&crossbow), 3);
        let plain = StackData { item: "tconstruct:crossbow".into(), ..Default::default() };
        assert_eq!(volley_size(&plain), 1);
        let bow = StackData { item: "tconstruct:longbow".into(), enchantments: [("minecraft:multishot".to_string(), 1)].into_iter().collect(), ..Default::default() };
        assert_eq!(volley_size(&bow), 1, "a bow fires one arrow whatever its enchantments say");
    }
}
