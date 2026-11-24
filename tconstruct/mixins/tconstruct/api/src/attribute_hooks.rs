//! Reads the bound-but-unread attributes in their canon event handlers: the critical
//! damage on the critical-hit event, the jump boost on the jump event, and the jump count
//! (double jump) and use item speed on the movement-input event. The experience multiplier
//! has no host hook for the experience a block break or a death drops, so it stays unread
//! (see custom_attributes.rs).
//!
//! Each formula reads three shares, as the canon does: the attribute, the tool stat (the
//! held tool's USE_ITEM_SPEED over its default, for the use-item speed only) and the armor
//! stat (the worn pieces' TinkerDataKeys totals). The readers take the values the server
//! tick keeps, or the synced copy on a guest client (attribute_sync): a guest's own
//! movement reads are the synced values, and a guest's jump press goes to the server as a
//! control packet instead of resolving locally.
//!
//! The host refuses a player's set-velocity, so a motion change lands as a push on the
//! player's next tick: the jump event and the input event only record what is pending,
//! and the tick applies it as the set velocity minus the motion the tick measured (the
//! bounce handler's shape). The jump boost is additive — the canon adds it to the motion —
//! so it lands as a plain upward push; the double jump sets the whole velocity, so it
//! lands as the set velocity minus the measured motion. The double jump's four
//! happy-villager particles spawn at the player's feet with the sound, as one send of
//! four (the port spreads them by a Gaussian, not the canon's uniform ±0.25 — the
//! placement matches, the distribution does not).
//! SlimeKnights (MIT; see LICENSE).
use crate::custom_attributes::{self, CRITICAL_DAMAGE, JUMP_BOOST, JUMP_COUNT, USE_ITEM_SPEED};
use crate::{attribute_sync, material_sync, tool_stack};
use crate::tool_stats::Stat;
use pluto_lib::{CriticalHit, EntityReach, LivingJump, MovementInput, ParticleData, PlayerTick, ServerLivingEntityEvents, World};
use std::collections::BTreeMap;
use std::sync::Mutex;

/// The sound a double jump makes (`Sounds.EXTRA_JUMP`), at the canon volume and pitch.
const DOUBLE_JUMP_SOUND: &str = "tconstruct:extra_jump";
/// The upward velocity `LivingEntity.jumpFromGround` sets (0.42).
const JUMP_VELOCITY: f64 = 0.42;
/// The upward velocity a jump boost adds per point (`boost * 0.1`, canon `onLivingJump`).
const JUMP_BOOST_PER_POINT: f64 = 0.1;
/// The declared default of the critical damage, from `custom_attributes::DECLARED`: the
/// boost is the value over this.
const CRITICAL_DAMAGE_DEFAULT: f64 = 1.5;
/// The declared default of the use item speed, from `custom_attributes::DECLARED` and the
/// tool stat's registered default: the tool-stat delta is the held tool's stat over this.
const USE_ITEM_SPEED_DEFAULT: f32 = 0.2;
/// The armor-stat key for the critical damage (TinkerDataKeys).
const CRITICAL_DAMAGE_STAT: &str = "tconstruct:critical_damage";
/// The armor-stat key for the jump boost (TinkerDataKeys).
const JUMP_BOOST_STAT: &str = "tconstruct:jump_boost";
/// The armor-stat key for the use item speed (TinkerDataKeys).
const USE_ITEM_SPEED_STAT: &str = "tconstruct:use_item_speed";
/// The double jump's particles (canon `DoubleJumpHandler.extraJump`): four happy villagers
/// at the player's feet, ±0.25 on x and z, no vertical spread, no speed.
const DOUBLE_JUMP_PARTICLE: &str = "minecraft:happy_villager";
const DOUBLE_JUMP_PARTICLE_COUNT: i32 = 4;
const DOUBLE_JUMP_PARTICLE_SPREAD: [f64; 3] = [0.25, 0.0, 0.25];

/// The jump boost to add on a player's next tick, by entity id: set by the jump event.
static PENDING_BOOST: Mutex<BTreeMap<i32, f64>> = Mutex::new(BTreeMap::new());
/// A double-jump attempt the input recorded, by entity id: resolved on the next tick,
/// with the fresh tick state (on ground, in water, jumps taken).
static PENDING_DOUBLE_JUMP: Mutex<BTreeMap<i32, ()>> = Mutex::new(BTreeMap::new());
/// The jumps taken this airtime, by entity id: reset on a ground jump and on landing.
static JUMPS_TAKEN: Mutex<BTreeMap<i32, i32>> = Mutex::new(BTreeMap::new());
/// Whether the jump key was held on the last input, by entity id: a double jump fires
/// once per press, not once per tick held.
static PREVIOUS_JUMP_KEY: Mutex<BTreeMap<i32, bool>> = Mutex::new(BTreeMap::new());
/// The position a player's last tick saw, by entity id: the tick's motion is the delta.
static KEPT: Mutex<BTreeMap<i32, [f64; 3]>> = Mutex::new(BTreeMap::new());

/// The critical boost: the critical damage over its declared default, plus the armor stat.
fn critical_boost(value: f64, armor_stat: f64) -> f64 {
    value - CRITICAL_DAMAGE_DEFAULT + armor_stat
}

/// Whether the hit is forced critical: the boost is positive, the attacker is airborne
/// (off the ground and not in water, as the port's airborne predicate reads it) and the
/// hit is not already critical.
fn force_critical(boost: f64, on_ground: bool, in_water: bool, already_critical: bool) -> bool {
    boost > 0.0 && !on_ground && !in_water && !already_critical
}

/// The extra jumps a jump count allows: the count less one, floored, never below zero
/// (the count's default of 1.0 is a single jump only).
fn extra_jumps(jump_count: f64) -> i32 {
    (jump_count.floor() as i32).saturating_sub(1).max(0)
}

/// Whether a double jump may fire: airborne (not on the ground, not in water — climbable
/// has no surface and reads as false, the bounce handler's precedent) with jumps left.
fn may_double_jump(on_ground: bool, in_water: bool, taken: i32, extra: i32) -> bool {
    !on_ground && !in_water && taken < extra
}

/// The factor the impulses are scaled by for a use item speed: the attribute, plus the
/// held tool's use-item-speed stat over its default, plus the armor stat, clamped to 0..1,
/// times 5 (which cancels vanilla's 20% slowdown, as the canon multiplies by `speed * 5`).
fn use_item_speed_factor(attribute: f64, tool_delta: f64, armor_stat: f64) -> f64 {
    (attribute + tool_delta + armor_stat).clamp(0.0, 1.0) * 5.0
}

/// The push a jump boost adds: upward by the boost, the canon `delta.add(0, boost*0.1, 0)`.
fn jump_push(boost: f64) -> [f64; 3] {
    [0.0, boost * JUMP_BOOST_PER_POINT, 0.0]
}

/// The velocity a double jump sets: the jump's own 0.42 plus the jump boost, with the
/// horizontal motion zeroed (canon `jumpFromGround` composed with `onLivingJump`).
fn double_jump_velocity(boost: f64) -> [f64; 3] {
    [0.0, JUMP_VELOCITY + boost * JUMP_BOOST_PER_POINT, 0.0]
}

/// The component-wise difference of two velocities.
fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

/// `ModifierEvents.onCritical` (CriticalHitEvent ~L293): the boost is the critical damage
/// over its default, plus the armor stat; while it is positive, an airborne hit is forced
/// critical, and a critical hit's damage multiplier grows by the boost. A hit the game runs
/// for no acting player (a guest's hit on a LAN world) names -1 and is left alone.
fn on_critical_hit(hit: &CriticalHit) {
    if hit.attacker_entity < 0 { return; }
    let boost = critical_boost(
        custom_attributes::value(hit.attacker_entity, CRITICAL_DAMAGE),
        custom_attributes::armor_stat_for(hit.attacker_entity, CRITICAL_DAMAGE_STAT),
    );
    if boost <= 0.0 { return; }
    if force_critical(boost, hit.attacker_on_ground, hit.attacker_in_water, hit.critical.get()) {
        hit.critical.set(true);
    }
    if hit.critical.get() {
        hit.damage_multiplier.set(hit.damage_multiplier.get() + boost as f32);
    }
}

/// `ModifierEvents.onLivingJump` (LivingJumpEvent ~L149): the jump resets the double-jump
/// counter (the original's `onJump`, which resets when the jump leaves the ground), and
/// the jump boost is added on the next tick. Only players are handled: the pending push
/// lands on a player tick, and a mob's value is the default (no boost) anyway.
fn on_living_jump(jump: &LivingJump) {
    if jump.kind != "minecraft:player" { return; }
    if let Ok(mut taken) = JUMPS_TAKEN.lock() { taken.remove(&jump.entity); }
    let boost = custom_attributes::value(jump.entity, JUMP_BOOST)
        + custom_attributes::armor_stat_for(jump.entity, JUMP_BOOST_STAT);
    if boost > 0.0 {
        if let Ok(mut pending) = PENDING_BOOST.lock() { pending.insert(jump.entity, boost); }
    }
}

/// `ToolClientEvents.handleInput` (MovementInputUpdateEvent ~L359-366): while the player
/// uses an item, the impulses are scaled by the use item speed (the attribute, plus the held
/// tool's use-item-speed stat over its default, plus the armor stat); a fresh press of the
/// jump key records a double-jump attempt, resolved on the next tick. A guest client sends
/// the attempt to the server instead of resolving it locally — it has no server tick — while
/// the owner resolves it locally. The entity reach is not open here, so nothing is pushed
/// from this callback. An off-hand use reads as the main hand's stack (the input's `using`
/// is the held stack; see entity_events_v1).
fn on_movement_input(input: &MovementInput) {
    if input.entity < 0 { return; }
    if input.using_item {
        // The tool-stat delta rides the input's used stack: the held tool's built stat, or
        // the registered default when the stack carries none (a plain item). A non-tool
        // contributes nothing.
        let tool_delta = input.using.as_ref().map(|stack| f64::from(tool_stack::stat(&stack.custom_data, Stat::UseItemSpeed) - USE_ITEM_SPEED_DEFAULT)).unwrap_or(0.0);
        let factor = use_item_speed_factor(
            custom_attributes::value(input.entity, USE_ITEM_SPEED),
            tool_delta,
            custom_attributes::armor_stat_for(input.entity, USE_ITEM_SPEED_STAT),
        );
        input.forward.set(input.forward.get() * factor as f32);
        input.strafe.set(input.strafe.get() * factor as f32);
    }
    let pressed = input.jump.get();
    let was_pressed = PREVIOUS_JUMP_KEY.lock().map(|mut keys| keys.insert(input.entity, pressed).unwrap_or(false)).unwrap_or(false);
    if pressed && !was_pressed {
        // Only when the player has jumps to spend: a press with no double-jump gear would
        // otherwise sit pending until they equip some, and fire from a stale press.
        if extra_jumps(custom_attributes::value(input.entity, JUMP_COUNT)) > 0 {
            if material_sync::remote_connection() {
                attribute_sync::request_double_jump();
            } else if let Ok(mut pending) = PENDING_DOUBLE_JUMP.lock() { pending.insert(input.entity, ()); }
        }
    }
}

/// The player's tick: the motion is measured (the position delta since the last tick),
/// landing resets the double-jump counter (the original's `onLand`), a pending jump
/// boost lands as an upward push, and a pending double-jump attempt resolves with the
/// fresh tick state. A double jump that fires subsumes a pending boost — its velocity
/// already carries the boost — so the boost is dropped instead of double-counted.
fn on_player_tick(tick: &PlayerTick) {
    let velocity = {
        let mut kept = KEPT.lock().expect("the attribute hook state is never poisoned");
        let previous = kept.insert(tick.entity, tick.position).unwrap_or(tick.position);
        [tick.position[0] - previous[0], tick.position[1] - previous[1], tick.position[2] - previous[2]]
    };
    if tick.on_ground {
        if let Ok(mut taken) = JUMPS_TAKEN.lock() { taken.remove(&tick.entity); }
    }
    // A guest client's jump press arrives as a control packet: it lands here, as a pending
    // double jump resolved with the server's values and jumps-taken state.
    if attribute_sync::take_double_jump_request(tick.player) {
        if let Ok(mut pending) = PENDING_DOUBLE_JUMP.lock() { pending.insert(tick.entity, ()); }
    }
    let mut double_jumped = false;
    if PENDING_DOUBLE_JUMP.lock().map(|mut pending| pending.remove(&tick.entity).is_some()).unwrap_or(false) {
        let taken = JUMPS_TAKEN.lock().map(|taken| taken.get(&tick.entity).copied().unwrap_or(0)).unwrap_or(0);
        let extra = extra_jumps(custom_attributes::value(tick.entity, JUMP_COUNT));
        if may_double_jump(tick.on_ground, tick.in_water, taken, extra) {
            let boost = custom_attributes::value(tick.entity, JUMP_BOOST)
                + custom_attributes::armor_stat_for(tick.entity, JUMP_BOOST_STAT);
            EntityReach::push(tick.entity as u64, sub(double_jump_velocity(boost), velocity));
            if let Ok(mut jumps) = JUMPS_TAKEN.lock() { jumps.insert(tick.entity, taken + 1); }
            World::new(&tick.dimension).play_sound(DOUBLE_JUMP_SOUND, tick.position, 0.5, 0.5);
            // The canon's four happy-villager particles at the player's feet, as one send of
            // four: the port spreads them by a Gaussian, not the canon's uniform ±0.25.
            World::new(&tick.dimension).send_particles(DOUBLE_JUMP_PARTICLE, &ParticleData::Plain, tick.position, DOUBLE_JUMP_PARTICLE_SPREAD, 0.0, DOUBLE_JUMP_PARTICLE_COUNT);
            double_jumped = true;
        }
    }
    if double_jumped {
        if let Ok(mut pending) = PENDING_BOOST.lock() { pending.remove(&tick.entity); }
    } else if let Some(boost) = PENDING_BOOST.lock().map(|mut pending| pending.remove(&tick.entity)).unwrap_or(None) {
        if boost > 0.0 { EntityReach::push(tick.entity as u64, jump_push(boost)); }
    }
}

/// Registers the callbacks. Called once from the product's hooks, after the custom
/// attributes (the callbacks here read the values that tick keeps).
pub(crate) fn register() {
    ServerLivingEntityEvents::CRITICAL_HIT.register(|hit| on_critical_hit(hit));
    ServerLivingEntityEvents::LIVING_JUMP.register(|jump| on_living_jump(jump));
    ServerLivingEntityEvents::MOVEMENT_INPUT.register(|input| on_movement_input(input));
    ServerLivingEntityEvents::PLAYER_TICK.register(|tick| on_player_tick(tick));
}

#[cfg(test)]
mod tests {
    use super::*;
    use pluto_lib::{Identifier, LivingFacts, MemoryEntityOps, MemoryWorldOps, WorldEntity, with_entities, with_world};
    use std::cell::{Cell, RefCell};

    #[test]
    fn the_critical_boost_is_the_value_over_its_default() {
        assert_eq!(critical_boost(1.5, 0.0), 0.0, "the default is no boost");
        assert_eq!(critical_boost(2.0, 0.0), 0.5);
        assert_eq!(critical_boost(1.0, 0.0), -0.5, "below the default, no force");
        assert_eq!(critical_boost(1.5, 0.25), 0.25, "the armor stat adds to the boost");
    }

    #[test]
    fn a_critical_hit_is_forced_only_airborne_with_a_boost() {
        assert!(force_critical(0.5, false, false, false));
        assert!(!force_critical(0.5, true, false, false), "on the ground, no force");
        assert!(!force_critical(0.5, false, true, false), "in water, no force");
        assert!(!force_critical(0.5, false, false, true), "already critical, no force");
        assert!(!force_critical(0.0, false, false, false), "no boost, no force");
    }

    #[test]
    fn the_extra_jumps_are_the_count_less_one_floored() {
        assert_eq!(extra_jumps(1.0), 0, "the default count is a single jump");
        assert_eq!(extra_jumps(2.0), 1);
        assert_eq!(extra_jumps(2.9), 1, "floored");
        assert_eq!(extra_jumps(0.5), 0, "never below zero");
    }

    #[test]
    fn a_double_jump_needs_air_and_jumps_left() {
        assert!(may_double_jump(false, false, 0, 1));
        assert!(!may_double_jump(true, false, 0, 1), "on the ground, no double jump");
        assert!(!may_double_jump(false, true, 0, 1), "in water, no double jump");
        assert!(!may_double_jump(false, false, 1, 1), "no jumps left");
    }

    #[test]
    fn the_use_item_speed_factor_cancels_vanillas_slowdown_at_default() {
        assert_eq!(use_item_speed_factor(0.2, 0.0, 0.0), 1.0, "the default 0.2 times 5 is full speed");
        assert_eq!(use_item_speed_factor(0.0, 0.0, 0.0), 0.0, "stopped");
        assert_eq!(use_item_speed_factor(1.0, 0.0, 0.0), 5.0);
        assert_eq!(use_item_speed_factor(2.0, 0.0, 0.0), 5.0, "clamped at 1.0");
        assert_eq!(use_item_speed_factor(0.2, 0.2, 0.0), 2.0, "a tool 0.2 over its default doubles the factor");
        assert_eq!(use_item_speed_factor(0.2, 0.0, 0.1), 1.5, "the armor stat adds to the speed");
        assert_eq!(use_item_speed_factor(0.2, -0.3, 0.0), 0.0, "a slow tool clamps at zero");
    }

    #[test]
    fn the_jump_pushes_are_the_canon_velocities() {
        assert_eq!(jump_push(1.0), [0.0, 0.1, 0.0]);
        assert_eq!(double_jump_velocity(0.0), [0.0, 0.42, 0.0]);
        // 0.42 + 0.2 is not exactly 0.62 in binary, so the composed velocity is compared loosely.
        assert!((double_jump_velocity(2.0)[1] - 0.62).abs() < 1e-9, "the boost composes into the double jump: {:?}", double_jump_velocity(2.0));
    }

    fn tick_for(entity: i32, position: [f64; 3], on_ground: bool, effects: Vec<(String, i32, i32)>) -> PlayerTick {
        PlayerTick {
            player: entity as u128, entity, dimension: "minecraft:overworld".to_owned(),
            position, look: [0.0, 0.0, 1.0], creative: false, on_ground, sneaking: false, gliding: false, spectator: false, in_water: false, on_climbable: false,
            armor: Vec::new(), hands: Vec::new(), block_light: 0, sky_light: 0, sky_darken: 0, effects,
            remove_effects: RefCell::new(Vec::new()), attribute_additions: RefCell::new(Vec::new()), actions: RefCell::new(Vec::new()),
        }
    }

    fn input_for(entity: i32, jump: bool, using_item: bool) -> MovementInput {
        input_with(entity, jump, using_item, None)
    }

    fn input_with(entity: i32, jump: bool, using_item: bool, using: Option<pluto_lib::StackData>) -> MovementInput {
        MovementInput {
            entity, forward: Cell::new(1.0), strafe: Cell::new(0.4),
            jump: Cell::new(jump), sneak: Cell::new(false), using_item, using,
        }
    }

    #[test]
    fn a_critical_hit_with_no_boost_leaves_the_hit_alone() {
        register();
        custom_attributes::register();
        let hit = CriticalHit {
            attacker_entity: 7707, attacker_on_ground: false, attacker_in_water: false,
            target_kind: "minecraft:zombie".into(), vanilla_critical: false,
            critical: Cell::new(false), damage_multiplier: Cell::new(1.0),
        };
        ServerLivingEntityEvents::CRITICAL_HIT.invoke(&hit);
        assert!(!hit.critical.get(), "the default critical damage is no boost");
        assert_eq!(hit.damage_multiplier.get(), 1.0);
    }

    #[test]
    fn using_an_item_at_the_default_speed_keeps_full_impulses() {
        register();
        custom_attributes::register();
        let input = input_for(7708, false, true);
        ServerLivingEntityEvents::MOVEMENT_INPUT.invoke(&input);
        assert_eq!(input.forward.get(), 1.0, "the default 0.2 cancels vanilla's slowdown");
        assert!((input.strafe.get() - 0.4).abs() < 1e-9);
    }

    #[test]
    fn a_held_tool_over_its_use_item_speed_default_speeds_the_impulses() {
        register();
        custom_attributes::register();
        // The held bow's built stat is 0.4, 0.2 over the default: the factor is (0.2 + 0.2) * 5 = 2.
        let mut stats = pluto_lib::NbtCompound::new();
        stats.insert("tconstruct:use_item_speed".into(), pluto_lib::Nbt::Float(0.4));
        let mut custom_data = pluto_lib::NbtCompound::new();
        custom_data.insert("tic_stats".into(), pluto_lib::Nbt::Compound(stats));
        let bow = pluto_lib::StackData { item: "tconstruct:longbow".into(), count: 1, custom_data, ..pluto_lib::StackData::default() };
        let input = input_with(7708, false, true, Some(bow));
        ServerLivingEntityEvents::MOVEMENT_INPUT.invoke(&input);
        assert_eq!(input.forward.get(), 2.0, "the tool-stat delta joins the attribute: {:?}", input.forward.get());
        assert!((input.strafe.get() - 0.8).abs() < 1e-9);
        // No stack in hand: the delta is zero and the default speed stands.
        let input = input_for(7708, false, true);
        ServerLivingEntityEvents::MOVEMENT_INPUT.invoke(&input);
        assert_eq!(input.forward.get(), 1.0);
    }

    #[test]
    fn a_double_jump_fires_once_per_press_and_once_per_airtime() {
        register();
        custom_attributes::register();
        let entity = 7709;
        // The double-jump effect grants one jump count: the count is 2, one extra jump.
        ServerLivingEntityEvents::PLAYER_TICK.invoke(&tick_for(entity, [0.0, 64.0, 0.0], true, vec![("tconstruct:double_jump".to_owned(), 0, 100)]));
        assert!((custom_attributes::value(entity, JUMP_COUNT) - 2.0).abs() < 1e-9, "the double-jump effect is read");
        let mut ops = MemoryEntityOps::default();
        ops.entities = vec![([0.0, 64.0, 0.0], WorldEntity { id: 7709, kind: Identifier::parse("minecraft:player").unwrap(), item: None, living: Some(LivingFacts { fire_immune: false, invulnerable: false, fire_resistant: false }) })];
        let mut world = MemoryWorldOps::default();
        // The recorded facts are asserted between rounds, not inside the scopes: while a
        // scope runs it holds the ops and the world mutably, so reading them inside the
        // closure that installs them does not borrow-check.
        // A fresh press in the air: the double jump fires.
        with_entities(&mut ops, || with_world(&mut world, || {
            ServerLivingEntityEvents::MOVEMENT_INPUT.invoke(&input_for(entity, true, false));
            ServerLivingEntityEvents::PLAYER_TICK.invoke(&tick_for(entity, [0.0, 64.0, 0.0], false, vec![("tconstruct:double_jump".to_owned(), 0, 100)]));
        }));
        assert_eq!(ops.pushed.len(), 1, "one push for the double jump");
        assert_eq!(ops.pushed[0].0, 7709);
        assert_eq!(ops.pushed[0].1, [0.0, 0.42, 0.0], "the set velocity, the measured motion being still: {:?}", ops.pushed[0].1);
        assert_eq!(world.sounds, vec![("tconstruct:extra_jump".to_owned(), [0.0, 64.0, 0.0])], "the extra-jump sound");
        assert_eq!(world.particles.len(), 1, "one send for the four particles");
        assert_eq!(world.particles[0].particle, "minecraft:happy_villager");
        assert_eq!((world.particles[0].at, world.particles[0].spread, world.particles[0].speed, world.particles[0].count), ([0.0, 64.0, 0.0], [0.25, 0.0, 0.25], 0.0, 4), "four at the feet, ±0.25 on x and z: {:?}", world.particles[0]);
        // Held down, the key is no fresh press: nothing more fires.
        with_entities(&mut ops, || with_world(&mut world, || {
            ServerLivingEntityEvents::MOVEMENT_INPUT.invoke(&input_for(entity, true, false));
            ServerLivingEntityEvents::PLAYER_TICK.invoke(&tick_for(entity, [0.0, 64.0, 0.0], false, vec![("tconstruct:double_jump".to_owned(), 0, 100)]));
        }));
        assert_eq!(ops.pushed.len(), 1, "a held key is not a fresh press");
        // Released and pressed again: the airtime has no jumps left.
        with_entities(&mut ops, || with_world(&mut world, || {
            ServerLivingEntityEvents::MOVEMENT_INPUT.invoke(&input_for(entity, false, false));
            ServerLivingEntityEvents::MOVEMENT_INPUT.invoke(&input_for(entity, true, false));
            ServerLivingEntityEvents::PLAYER_TICK.invoke(&tick_for(entity, [0.0, 64.0, 0.0], false, vec![("tconstruct:double_jump".to_owned(), 0, 100)]));
        }));
        assert_eq!(ops.pushed.len(), 1, "one extra jump per airtime");
        // Landing resets the counter: a fresh press jumps again.
        with_entities(&mut ops, || with_world(&mut world, || {
            ServerLivingEntityEvents::PLAYER_TICK.invoke(&tick_for(entity, [0.0, 64.0, 0.0], true, vec![("tconstruct:double_jump".to_owned(), 0, 100)]));
            ServerLivingEntityEvents::MOVEMENT_INPUT.invoke(&input_for(entity, false, false));
            ServerLivingEntityEvents::MOVEMENT_INPUT.invoke(&input_for(entity, true, false));
            ServerLivingEntityEvents::PLAYER_TICK.invoke(&tick_for(entity, [0.0, 64.0, 0.0], false, vec![("tconstruct:double_jump".to_owned(), 0, 100)]));
        }));
        assert_eq!(ops.pushed.len(), 2, "landing resets the counter");
    }

    #[test]
    fn a_pending_jump_boost_pushes_up_on_the_next_tick() {
        register();
        let entity = 7710;
        if let Ok(mut pending) = PENDING_BOOST.lock() { pending.insert(entity, 1.0); }
        let mut ops = MemoryEntityOps::default();
        ops.entities = vec![([0.0, 64.0, 0.0], WorldEntity { id: 7710, kind: Identifier::parse("minecraft:player").unwrap(), item: None, living: Some(LivingFacts { fire_immune: false, invulnerable: false, fire_resistant: false }) })];
        with_entities(&mut ops, || {
            ServerLivingEntityEvents::PLAYER_TICK.invoke(&tick_for(entity, [0.0, 64.0, 0.0], true, Vec::new()));
        });
        assert_eq!(ops.pushed.len(), 1, "the boost lands as one push");
        assert_eq!(ops.pushed[0].1, [0.0, 0.1, 0.0], "a boost of 1.0 pushes up by 0.1: {:?}", ops.pushed[0].1);
        with_entities(&mut ops, || {
            ServerLivingEntityEvents::PLAYER_TICK.invoke(&tick_for(entity, [0.0, 64.0, 0.0], true, Vec::new()));
        });
        assert_eq!(ops.pushed.len(), 1, "the boost is spent");
    }
}
