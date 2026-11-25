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
//! (pierce, no gravity, the angle fan) whether or not a profile is bound yet, and marks an arrow
//! the Infinity module supplied creative-only pickup, so it cannot be picked up again. The weapon's
//! own Infinity reaches the same arrow through the flight pass: the shot binds the weapon's profile,
//! and the first flight tick sets the pickup, for an arrow the shooter carried.
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
    ActionTarget, ArrowPickup, EntityAction, EntityReach, ExplosionInteraction, FlightAction, ItemSlot, KnockbackDirection,
    Nbt, NbtCompoundExt, ProjectileEvents, ProjectileFlight, ProjectileImpact, ProjectileSpawned, StackData, StackSlot,
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
    /// Whether the weapon-side Infinity mark has been applied to this flight's arrow already (the
    /// flight pass sets it once, on the first tick it sees the bound profile).
    pickup_set: bool,
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
        infinity: ammo.infinity || weapon.infinity,
        tipped: weapon.tipped.or(ammo.tipped),
        spilling: [ammo.spilling, weapon.spilling].concat(),
    }
}

/// The spawn pass: a shot is set up from the stack it was fired as (its own modifiers), whether or not
/// a profile is bound under its id yet.
fn on_spawn(spawn: &ProjectileSpawned) {
    let Some(stack) = &spawn.stack else { return };
    // InfinityModule.onProjectileShoot: an arrow the modifier supplied carries the tic_infinity mark,
    // and the original sets the fired arrow to creative-only pickup, so a supplied arrow that lands
    // cannot be picked up again. The mark is read here, before the profile's early return, because a
    // supplied arrow is a plain stack with no modifiers of its own.
    if matches!(stack.custom_data.get("tic_infinity"), Some(Nbt::Byte(1)) | Some(Nbt::Int(1))) {
        spawn.pickup.set(Some(ArrowPickup::CreativeOnly));
    }
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

/// The punch level of the bow or crossbow that fired the arrow, read off the saved data: the
/// arrow's save carries no knockback of its own, but it carries the weapon that fired it (`weapon`,
/// as the game saves it, with the weapon's enchantments under `components.minecraft:enchantments`),
/// and the game's own arrow hit reads punch off that weapon. `None` for an arrow no weapon is saved
/// for (a skeleton's, a dispenser's): the game's own hit then falls back to the punch the shot was
/// fired with, which the hit reports as [`ProjectileImpact::shot_punch`] — a skeleton holding an
/// enchanted bow contributes its punch that way, though no save records its weapon.
fn arrow_punch(data: &pluto_lib::NbtCompound) -> Option<i32> {
    let weapon = data.get_compound("weapon")?;
    let punch = weapon
        .get_compound("components")
        .and_then(|components| components.get_compound("minecraft:enchantments"))
        .map(|enchantments| enchantments.get_int("minecraft:punch", 0))
        .unwrap_or(0);
    Some(punch.clamp(0, i64::from(i32::MAX)) as i32)
}

/// The push the original's punch deals on a hit: along the arrow's horizontal motion, normalized
/// and scaled by punch × 0.6, and up by a fixed 0.1 (`target.push(knockbackVec.x, 0.1D,
/// knockbackVec.z)`) — a push, not a knockback, so the lift is 0.1 however strong the punch is,
/// and no knockback resistance is taken off it. None when the arrow has no horizontal motion to
/// push along (the original checks the scaled vector's length the same way).
fn punch_push(motion: [f64; 3], punch: i32) -> Option<[f64; 3]> {
    let length = motion[0].hypot(motion[2]);
    if !(length.is_finite() && length >= 1.0e-5) { return None; }
    let scale = f64::from(punch) * 0.6 / length;
    Some([motion[0] * scale, 0.1, motion[2] * scale])
}

/// Deals ModifierEvents' reimplemented arrow hit on the impact's target: the arrow's own damage
/// as the melee type, the punch push the original adds, the target lit for 5 seconds when the
/// arrow burns, and the arrow spent unless it pierces. A piercing arrow flies on, and the victim is
/// written to the game's ignore list, so it does not meet the same enderman again; an arrow that
/// has already passed through as many as its pierce allows is spent instead, and deals nothing.
/// The arrow's numbers are read off its saved data, as the game saves them: `damage` the base
/// damage (2.0 when the save names none, the game's own default), `crit`, `PierceLevel`, and
/// `Fire` the ticks it has left burning. The killed list (`piercedAndKilledEntities`, which feeds
/// the crossbow-kill criterion) is kept for the arrow by PlutoLib and reported on the hit as
/// [`ProjectileImpact::pierced_and_killed`]; a kill this hit deals joins it as the hit settles. One
/// reduction against the original remains: the punch push is one of the port's actions, so it
/// lands whether or not the hurt did, as the fire does, where the original pushes only on a hurt
/// that landed.
fn deal_arrow_override(hit: &ProjectileImpact) {
    let pierce = hit.data.get_int("PierceLevel", 0);
    if pierce > 0 {
        // The game's own tick discards a piercing arrow past its pierce before it tells the hit,
        // so this is a guard rather than the usual path — but the original checks the same bound,
        // and the snapshot already holds this victim, so the count to compare is its length less
        // the victim (the original counts the list before adding it).
        let passed = hit.piercing_ignored.len().saturating_sub(1);
        if passed as i64 >= pierce + 1 {
            hit.discard.set(true);
            return;
        }
        if let Some(victim) = hit.entity {
            hit.ignore_entities.borrow_mut().push(victim);
        }
    }
    let base_damage = hit.data.get_double("damage", 2.0);
    let critical = hit.data.get_int("crit", 0) != 0;
    let amount = arrow_hit_damage(hit.motion, base_damage, critical, &mut |bound| {
        (f64::from(tool_damage_hook::random()) * f64::from(bound.max(1))) as i32
    });
    let mut actions = hit.actions.borrow_mut();
    actions.push(EntityAction::Hurt { target: ActionTarget::Victim, amount, damage_type: MELEE_ARROW_DAMAGE.into() });
    let punch = arrow_punch(&hit.data).unwrap_or(hit.shot_punch);
    if punch > 0 {
        // The original pushes along the arrow's horizontal motion, by punch × 0.6, and up by a
        // fixed 0.1: a push, not a knockback, so the lift does not grow with the punch.
        if let Some(velocity) = punch_push(hit.motion, punch) {
            actions.push(EntityAction::Push { target: ActionTarget::Victim, velocity });
        }
    }
    if hit.data.get_int("Fire", 0) > 0 {
        actions.push(EntityAction::SetFire { target: ActionTarget::Victim, seconds: 5 });
    }
    drop(actions);
    if pierce <= 0 {
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
    // InfinityModule.onProjectileShoot, weapon side: the shot hooks run over the weapon, so an arrow
    // a player carried and fired from an Infinity launcher is creative-only too — the spawn pass
    // cannot see the weapon (it reads the ammunition's own stack), so the first flight tick applies
    // it once the shot has bound the weapon's profile. A thrown tool or shuriken is not a shot: its
    // pickup is its own stack, handed back where it lands.
    if profile.infinity && !state.pickup_set && state.thrown.is_none() && state.shuriken.is_none() {
        flight.pickup_after.set(Some(ArrowPickup::CreativeOnly));
        state.pickup_set = true;
    }
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
            // (updateFishingRod: the wear is queued here and lands on the owner's next
            // tick, which starts the draw-speed cooldown as it lands).
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
    // bobber wears its rod by 10 on a swap that lands (updateFishingRod; the queued wear
    // starts the draw-speed cooldown when it lands — see the fuse branch above).
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
    // has no amount at the impact telling; a hit on a living target that can be hurt deals
    // damage, so that is the gate (an invulnerable or non-living target deals none).
    // A thrown tool's own impact never reaches this block (see the thrown-tool branch above),
    // so its spilling stays unwired there — out of this lane's four items, noted for the next.
    if hit.entity.is_some() && !profile.spilling.is_empty() && hit.entity.is_some_and(|id| {
        EntityReach::in_box([hit.at[0] - 0.5, hit.at[1] - 0.5, hit.at[2] - 0.5], [hit.at[0] + 0.5, hit.at[1] + 0.5, hit.at[2] + 0.5])
            .into_iter().find(|entity| entity.id == id as u64)
            .is_some_and(|entity| entity.living.is_some_and(|facts| !facts.invulnerable))
    }) {
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
        // whichever stack carries them, as the shot hooks run over both; so does the weapon's
        // Infinity, for the fired arrow's pickup.
        let ammo = tool_stack::FlightProfile { efln: true, ..Default::default() };
        let weapon = tool_stack::FlightProfile { enderference: true, infinity: true, ..Default::default() };
        let merged = merge(ammo, weapon);
        assert!(merged.efln);
        assert!(merged.enderference);
        assert!(merged.infinity);
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
            piercing_ignored: Vec::new(),
            pierced_and_killed: Vec::new(),
            shot_punch: 0,
            discard: std::cell::Cell::new(false),
            motion_after: std::cell::RefCell::new(None),
            ignore_entities: std::cell::RefCell::new(Vec::new()),
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
            piercing_ignored: Vec::new(),
            pierced_and_killed: Vec::new(),
            shot_punch: 0,
            discard: std::cell::Cell::new(false),
            motion_after: std::cell::RefCell::new(None),
            ignore_entities: std::cell::RefCell::new(Vec::new()),
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
        // to gate on, so its hooks run too. The effect case's hit is answered by the arrow
        // reimplementation instead of the game's own hit (see
        // `a_bound_arrow_is_reimplemented_after_its_hit_hooks`), so it returns false while its
        // hooks still ran.
        for (dimension, entity, kind, effects, mark, lands) in [
            ("test:endergate-mark", Some(9), "minecraft:enderman", Vec::new(), true, true),
            ("test:endergate-effect", Some(9), "minecraft:enderman", vec![crate::teleport_dodge::ENDERFERENCE.to_owned()], false, false),
            ("test:endergate-zombie", Some(9), "minecraft:zombie", Vec::new(), false, true),
            ("test:endergate-block", None, "", Vec::new(), false, true),
        ] {
            let hit = gated_hit(dimension, entity, kind, effects, mark);
            assert_eq!(on_impact(&hit), lands, "{dimension}: the hit lands as the game's own, except where the reimplementation answers it");
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
    /// named base damage, crit flag, pierce level and remaining fire ticks, the named punch level
    /// on the bow that fired it, and the named ignore list the game reports for it. `bound` first
    /// binds an attracting flight under the projectile (the one hook whose firing is visible
    /// without the world), as a shot with a profile would.
    fn arrow_hit(dimension: &str, kind: &str, entity_kind: &str, effects: Vec<String>, base_damage: f64, critical: bool, pierce: i8, fire: i16, punch: i32, piercing_ignored: Vec<u32>, bound: bool) -> ProjectileImpact {
        if bound {
            bind(dimension, 7, InFlight { profile: tool_stack::FlightProfile { attracts: vec![(3.0, 0.5)], ..Default::default() }, ..Default::default() });
        }
        let mut data = pluto_lib::NbtCompound::new();
        data.insert("damage".into(), pluto_lib::Nbt::Double(base_damage));
        data.insert("crit".into(), pluto_lib::Nbt::Byte(if critical { 1 } else { 0 }));
        data.insert("PierceLevel".into(), pluto_lib::Nbt::Byte(pierce));
        data.insert("Fire".into(), pluto_lib::Nbt::Short(fire));
        if punch > 0 {
            let mut enchantments = pluto_lib::NbtCompound::new();
            enchantments.insert("minecraft:punch".into(), pluto_lib::Nbt::Int(punch));
            let mut components = pluto_lib::NbtCompound::new();
            components.insert("minecraft:enchantments".into(), pluto_lib::Nbt::Compound(enchantments));
            let mut weapon = pluto_lib::NbtCompound::new();
            weapon.insert("components".into(), pluto_lib::Nbt::Compound(components));
            data.insert("weapon".into(), pluto_lib::Nbt::Compound(weapon));
        }
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
            piercing_ignored,
            pierced_and_killed: Vec::new(),
            shot_punch: 0,
            discard: std::cell::Cell::new(false),
            motion_after: std::cell::RefCell::new(None),
            ignore_entities: std::cell::RefCell::new(Vec::new()),
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

    /// The pushes an impact dealt, as velocity vectors.
    fn dealt_pushes(hit: &ProjectileImpact) -> Vec<[f64; 3]> {
        hit.actions.borrow().iter().filter_map(|action| match action {
            EntityAction::Push { velocity, .. } => Some(*velocity),
            _ => None,
        }).collect()
    }

    #[test]
    fn an_unbound_arrow_is_reimplemented_on_an_exposed_enderman() {
        // A skeleton's arrow, no flight bound: ModifierEvents still answers, dealing the arrow's
        // damage (speed 3 × base 2 = 6) as the melee type, and the arrow is spent.
        let hit = arrow_hit("test:enderarrow-unbound", "minecraft:arrow", "minecraft:enderman", vec![crate::teleport_dodge::ENDERFERENCE.to_owned()], 2.0, false, 0, 0, 0, Vec::new(), false);
        assert!(!on_impact(&hit), "the reimplementation replaces the game's hit");
        assert_eq!(dealt_hurts(&hit), vec![(MELEE_ARROW_DAMAGE.to_owned(), 6.0)]);
        assert!(hit.discard.get(), "a non-piercing arrow is spent");
    }

    #[test]
    fn a_bound_arrow_is_reimplemented_after_its_hit_hooks() {
        // A shot with a profile: the hit hooks run (the attraction queues), then the
        // reimplementation deals the damage; a burning arrow lights the target for 5 seconds.
        let dimension = "test:enderarrow-bound";
        let hit = arrow_hit(dimension, "minecraft:spectral_arrow", "minecraft:enderman", vec![crate::teleport_dodge::ENDERFERENCE.to_owned()], 2.0, false, 0, 100, 0, Vec::new(), true);
        assert!(!on_impact(&hit), "the reimplementation replaces the game's hit");
        assert_eq!(dealt_hurts(&hit), vec![(MELEE_ARROW_DAMAGE.to_owned(), 6.0)]);
        assert!(hit.actions.borrow().iter().any(|action| matches!(action, EntityAction::SetFire { seconds: 5, .. })), "a burning arrow lights its target");
        let mut guard = PENDING_ATTRACTION.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let queued = guard.get_or_insert_with(HashMap::new).remove(dimension).unwrap_or_default();
        assert_eq!(queued.len(), 1, "the attraction hook ran before the reimplementation");
    }

    #[test]
    fn a_piercing_arrow_flies_on_after_the_reimplemented_hit() {
        let hit = arrow_hit("test:enderarrow-pierce", "minecraft:arrow", "minecraft:enderman", vec![crate::teleport_dodge::ENDERFERENCE.to_owned()], 2.0, false, 1, 0, 0, vec![9], false);
        assert!(!on_impact(&hit), "the reimplementation replaces the game's hit");
        assert_eq!(dealt_hurts(&hit), vec![(MELEE_ARROW_DAMAGE.to_owned(), 6.0)]);
        assert!(!hit.discard.get(), "a piercing arrow is not spent");
        assert_eq!(
            hit.ignore_entities.borrow().as_slice(),
            [9],
            "the victim is recorded for the game's ignore list, so the arrow does not meet it again"
        );
    }

    #[test]
    fn a_piercing_arrow_past_its_pierce_is_spent_without_dealing() {
        // Pierce 1, and the snapshot already holds two passed through plus this victim: the
        // original spends the arrow instead of dealing (its list had reached pierce + 1).
        let hit = arrow_hit("test:enderarrow-pierce-full", "minecraft:arrow", "minecraft:enderman", vec![crate::teleport_dodge::ENDERFERENCE.to_owned()], 2.0, false, 1, 0, 0, vec![7, 8, 9], false);
        assert!(!on_impact(&hit), "the hit is still cancelled, the game's own never runs");
        assert!(dealt_hurts(&hit).is_empty(), "nothing is dealt once the pierce is spent");
        assert!(hit.ignore_entities.borrow().is_empty(), "a spent arrow records nothing");
        assert!(hit.discard.get(), "the arrow is spent");
    }

    #[test]
    fn a_punch_arrow_pushes_its_target_along_its_flight_with_a_fixed_lift() {
        // Punch II on the bow: the original pushes by 2 × 0.6 along the arrow's motion and up by
        // a fixed 0.1 — a push, not a knockback, so the lift is 0.1, not min(0.4, 1.2).
        let hit = arrow_hit("test:enderarrow-punch", "minecraft:arrow", "minecraft:enderman", vec![crate::teleport_dodge::ENDERFERENCE.to_owned()], 2.0, false, 0, 0, 2, Vec::new(), false);
        assert!(!on_impact(&hit), "the reimplementation replaces the game's hit");
        assert_eq!(dealt_hurts(&hit), vec![(MELEE_ARROW_DAMAGE.to_owned(), 6.0)]);
        let pushes = dealt_pushes(&hit);
        assert_eq!(pushes.len(), 1, "one push is dealt");
        let [x, y, z] = pushes[0];
        assert!(x.abs() < 1.0e-9 && (z + 1.2).abs() < 1.0e-9, "the push runs along the arrow's flight, at punch × 0.6: {pushes:?}");
        assert_eq!(y, 0.1, "the lift is the original's fixed 0.1");
    }

    #[test]
    fn a_skeletons_enchanted_bow_contributes_its_punch() {
        // No weapon is saved on a skeleton's arrow, so the punch comes from the shot's own
        // recorded bonus, as the game's own hit falls back to it: Punch II pushes the same way.
        let mut hit = arrow_hit("test:enderarrow-skeleton-punch", "minecraft:arrow", "minecraft:enderman", vec![crate::teleport_dodge::ENDERFERENCE.to_owned()], 2.0, false, 0, 0, 0, Vec::new(), false);
        hit.shot_punch = 2;
        assert!(!on_impact(&hit), "the reimplementation replaces the game's hit");
        let pushes = dealt_pushes(&hit);
        assert_eq!(pushes.len(), 1, "the skeleton's punch pushes too");
        assert_eq!(pushes[0][1], 0.1, "with the same fixed lift");
        assert!((pushes[0][2] + 1.2).abs() < 1.0e-9, "at punch × 0.6 along the flight: {pushes:?}");
    }

    #[test]
    fn a_saved_weapons_punch_wins_over_the_shots_recorded_punch() {
        // The game's own hit reads the weapon when one is saved and the shot's bonus only when
        // none is: Punch I on the saved bow governs, not the recorded Punch II.
        let mut hit = arrow_hit("test:enderarrow-weapon-wins", "minecraft:arrow", "minecraft:enderman", vec![crate::teleport_dodge::ENDERFERENCE.to_owned()], 2.0, false, 0, 0, 1, Vec::new(), false);
        hit.shot_punch = 2;
        assert!(!on_impact(&hit), "the reimplementation replaces the game's hit");
        let pushes = dealt_pushes(&hit);
        assert_eq!(pushes.len(), 1);
        assert!((pushes[0][2] + 0.6).abs() < 1.0e-9, "the push is punch I's, 1 × 0.6 along the flight: {pushes:?}");
    }

    #[test]
    fn the_punch_push_runs_along_the_horizontal_motion_only() {
        // A purely vertical shot has nothing to push along; a diagonal is normalized first, and
        // the vertical part of the flight never enters the push.
        assert!(punch_push([0.0, -3.0, 0.0], 2).is_none());
        let push = punch_push([3.0, -1.0, 4.0], 1).expect("a diagonal pushes");
        assert!((push[0] - 0.36).abs() < 1.0e-9 && push[1] == 0.1 && (push[2] - 0.48).abs() < 1.0e-9, "{push:?}");
    }

    #[test]
    fn an_arrow_without_punch_deals_no_push() {
        let hit = arrow_hit("test:enderarrow-no-punch", "minecraft:arrow", "minecraft:enderman", vec![crate::teleport_dodge::ENDERFERENCE.to_owned()], 2.0, false, 0, 0, 0, Vec::new(), false);
        assert!(!on_impact(&hit), "the reimplementation replaces the game's hit");
        assert!(dealt_pushes(&hit).is_empty(), "no punch on the weapon or the shot, no push");
    }

    #[test]
    fn punch_is_read_off_the_weapon_that_fired_the_arrow() {
        // The weapon's enchantments, as the game saves them; an arrow no weapon is saved for
        // reads None, so the caller falls back to the shot's recorded punch.
        let mut weapon = pluto_lib::NbtCompound::new();
        let mut components = pluto_lib::NbtCompound::new();
        let mut enchantments = pluto_lib::NbtCompound::new();
        enchantments.insert("minecraft:punch".into(), pluto_lib::Nbt::Int(1));
        components.insert("minecraft:enchantments".into(), pluto_lib::Nbt::Compound(enchantments));
        weapon.insert("components".into(), pluto_lib::Nbt::Compound(components));
        let mut data = pluto_lib::NbtCompound::new();
        data.insert("weapon".into(), pluto_lib::Nbt::Compound(weapon));
        assert_eq!(arrow_punch(&data), Some(1));
        assert_eq!(arrow_punch(&pluto_lib::NbtCompound::new()), None, "an arrow no weapon is saved for");
        let mut bare = pluto_lib::NbtCompound::new();
        bare.insert("weapon".into(), pluto_lib::Nbt::Compound(pluto_lib::NbtCompound::new()));
        assert_eq!(arrow_punch(&bare), Some(0), "a weapon with no enchantments");
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
            let hit = arrow_hit(dimension, kind, entity_kind, effects, 2.0, false, 0, 0, 0, Vec::new(), false);
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

    #[test]
    fn a_supplied_arrow_is_set_creative_only_at_spawn_and_a_plain_arrow_is_left_alone() {
        use pluto_lib::{ArrowFacts, NbtCompound};
        use std::cell::{Cell, RefCell};
        let spawn_with = |stack: Option<StackData>| ProjectileSpawned {
            dimension: "minecraft:overworld".into(), projectile: 0, kind: "minecraft:arrow".into(), owner: 4,
            position: [0.5, 64.0, 0.5], motion: [0.0, 0.2, -2.0],
            arrow: Some(ArrowFacts { pierce: 0, in_ground: false, in_ground_time: 0, dealt_damage: false }),
            stack, data: NbtCompound::new(),
            motion_after: RefCell::new(None), pierce: Cell::new(None), pickup: Cell::new(None), no_gravity: Cell::new(None), pickup_stack: RefCell::new(None),
        };
        let mut supplied_data = NbtCompound::new();
        supplied_data.insert("tic_infinity".into(), Nbt::Byte(1));
        let supplied = spawn_with(Some(StackData { item: "minecraft:arrow".into(), count: 1, custom_data: supplied_data, ..StackData::default() }));
        on_spawn(&supplied);
        assert_eq!(supplied.pickup.get(), Some(ArrowPickup::CreativeOnly), "an Infinity-supplied arrow cannot be picked up again");
        let plain = spawn_with(Some(StackData { item: "minecraft:arrow".into(), count: 1, ..StackData::default() }));
        on_spawn(&plain);
        assert_eq!(plain.pickup.get(), None, "a plain arrow keeps the game's own pickup");
        let none = spawn_with(None);
        on_spawn(&none);
        assert_eq!(none.pickup.get(), None);
    }

    #[test]
    fn a_weapon_side_infinity_marks_the_arrow_once_on_its_first_flight() {
        use pluto_lib::{ArrowFacts, NbtCompound};
        use std::cell::{Cell, RefCell};
        let flight_at = |age: u32| ProjectileFlight {
            dimension: "minecraft:overworld".into(), projectile: 424242, kind: "minecraft:arrow".into(), owner: 4,
            owner_position: None, age, position: [0.5, 64.0, 0.5], motion: [0.0; 3],
            in_ground: false, no_gravity: false,
            arrow: Some(ArrowFacts { pierce: 0, in_ground: false, in_ground_time: 0, dealt_damage: false }),
            data: NbtCompound::new(),
            motion_after: RefCell::new(None), discard: Cell::new(false), no_gravity_after: Cell::new(None), pickup_after: Cell::new(None),
            release_from_ground: Cell::new(false), actions: RefCell::new(Vec::new()),
        };
        bind("minecraft:overworld", 424242, InFlight { profile: tool_stack::FlightProfile { infinity: true, ..Default::default() }, ..Default::default() });
        let first = flight_at(1);
        on_flight(&first);
        assert_eq!(first.pickup_after.get(), Some(ArrowPickup::CreativeOnly), "the weapon's Infinity marks a carried arrow");
        let second = flight_at(2);
        on_flight(&second);
        assert_eq!(second.pickup_after.get(), None, "the mark is applied once, on the first flight tick");
    }

    #[test]
    fn a_thrown_tool_with_infinity_keeps_its_own_pickup() {
        use pluto_lib::{ArrowFacts, NbtCompound};
        use std::cell::{Cell, RefCell};
        let flight = ProjectileFlight {
            dimension: "minecraft:overworld".into(), projectile: 424243, kind: "minecraft:arrow".into(), owner: 4,
            owner_position: None, age: 1, position: [0.5, 64.0, 0.5], motion: [0.0; 3],
            in_ground: false, no_gravity: false,
            arrow: Some(ArrowFacts { pierce: 0, in_ground: false, in_ground_time: 0, dealt_damage: false }),
            data: NbtCompound::new(),
            motion_after: RefCell::new(None), discard: Cell::new(false), no_gravity_after: Cell::new(None), pickup_after: Cell::new(None),
            release_from_ground: Cell::new(false), actions: RefCell::new(Vec::new()),
        };
        let stack = StackData { item: "example:thrown_tool".into(), count: 1, ..StackData::default() };
        bind("minecraft:overworld", 424243, thrown_flight(tool_stack::FlightProfile { infinity: true, ..Default::default() }, stack, 1.0));
        on_flight(&flight);
        assert_eq!(flight.pickup_after.get(), None, "a thrown tool is not a shot: its pickup is its own stack");
    }
}
