//! The sling release (the SlingModule family, `sling_modules`): charging a tool that
//! carries a sling module and letting go spends the charge as a force — half the tool's
//! projectile damage times its velocity, scaled by the module's charge and force
//! multiplier, reshaped by the tool's sling force modules — and the kind decides where
//! the force lands. SlingKnockbackModule bonks the living entity in front of the holder
//! with a real melee attack; SlingLeapModule launches the holder; SlingTeleportModule
//! moves the holder along the look to the furthest standable block the offset reaches.
//!
//! The push lands a tick late, by construction: `EntityReach::knockback`,
//! `EntityReach::push` and `EntityReach::leave_ground` queue the motion, and the host
//! applies it at its next rider synchronisation through the game's own knockback and
//! motion paths — the target's knockback resistance applies as the original's does, and
//! a pushed player's client is told. The forces and angles are computed here exactly as
//! the original computes them, and every effect around them (the bonk's damage, sounds,
//! particles, wear, cooldown) runs in the telling, as it does in the original.
//!
//! What has no surface yet, exactly:
//! - The teleport: `EntityReach::teleport` answers false for a player (the host has no
//!   call that moves a player), so the wiring stands ready and keys its success effects
//!   off the answer, as they key off the original's teleport event not being cancelled;
//!   it lands the day the player move does.
//! - The leap's slime bounce handler and a drill tool's spin attack now run: the leap adds the
//!   bounce handler (`crate::bounce::add_bounce_handler`, the momentum-only handler, which the
//!   fall handler then drives), and a drill sling starts the spin attack (`EntityReach::spin_attack`)
//!   when the leaping modifier is in the drill-attacks tag and the tool can perform the drill attack.
//! - Exhaustion (0.2 per sling): the player telling reads food, it does not spend it.
//! - The three-tick item cooldown: `ReleaseUsing::set_used` starts the item's own
//!   content cooldown instead.
//! - The helmet path (SlingModule's armor interact charging): no server telling carries
//!   a helmet interact, so slings fire from the hand only.
//! - The active modifier: the original remembers which modifier started the use and
//!   plays its failure sound; the port keeps no such record, so the first sling module
//!   in modifier order speaks for it.
//! - A bonk target's own ground and flight, which its predicate reads: the entity reach
//!   reports neither, so they read false (the shipped knockback module carries no target
//!   predicate). The holder's ground comes from the player tick; the holder's fall
//!   flying is not reported either, so springing's inverted elytra predicate passes.
use crate::{
    break_speed_hook::with_held_tool,
    modifier_entries::Entry,
    sling_modules::{LivingState, Sling},
    tool_slot, tool_stack,
    tool_stats::Stat,
};
use pluto_lib::{
    CarriedEdit, CarriedSlot, EntityReach, Identifier, ItemEvents, ItemSlot, NbtCompound,
    ParticleData, PlayerTick, ReleaseUsing, ServerLivingEntityEvents, StackSlot, World,
};
use std::collections::BTreeMap;
use std::sync::Mutex;

/// How far a bonk reaches (SlingKnockbackModule's RANGE).
const RANGE: f64 = 5.0;
/// Each player's last reported ground state, for the leap predicates (the release
/// telling does not carry it; the player tick does).
static ON_GROUND: Mutex<BTreeMap<u128, bool>> = Mutex::new(BTreeMap::new());

/// Each player's entity id, for the formulas that read the holder's own attributes
/// (the release telling names the player by uuid; their tick names the id).
static HOLDER_ENTITY: Mutex<BTreeMap<u128, i32>> = Mutex::new(BTreeMap::new());

/// Registers the tick cache and the release callback. Called once from the product's
/// hooks, after the throwing hook (whose use registration this composes into).
pub(crate) fn register() {
    ServerLivingEntityEvents::PLAYER_TICK.register(on_tick);
    ItemEvents::RELEASE_USING.register(on_release);
}

fn on_tick(tick: &PlayerTick) {
    if let Ok(mut map) = ON_GROUND.lock() {
        map.insert(tick.player, tick.on_ground);
    }
    if let Ok(mut map) = HOLDER_ENTITY.lock() {
        map.insert(tick.player, tick.entity);
    }
}

fn on_release(release: &ReleaseUsing) {
    // The sling force formulas read the holder's own attributes as of their last tick:
    // name the holder by the entity id their tick reported, for the evaluation below.
    let holder_entity = HOLDER_ENTITY.lock().ok().and_then(|map| map.get(&release.player.uuid).copied());
    crate::entity_facts::with_holder_entity(holder_entity, || release_with(release));
}

fn release_with(release: &ReleaseUsing) {
    let _ = with_held_tool(&release.stack, |env, tool| {
        let nbt = &tool.custom_data;
        let slings = tool_stack::sling_modules(nbt, env);
        if slings.is_empty() {
            return;
        }
        let holder = holder_state(release);
        // The holder's height, for the dwarven velocity half of the sling power; the build range of
        // the level they stand in, since a shot's holder is not digging and no dig reports it:
        // the host's own range for the dimension, the vanilla table when it has not answered.
        let holder_entity = pluto_lib::EntityState { on_ground: holder.on_ground, y: Some(release.position[1]), ..Default::default() };
        let entities = crate::entity_facts::Entities { attacker: Some(&holder_entity), target: None };
        let build_height = Some(crate::mining_speed::level_build_height(&release.dimension));
        let active = slings[0].1.clone();
        for (entry, id, module) in &slings {
            let charge = charge(release.ticks_used, drawtime(nbt, env, &entities, module));
            if charge <= 0.0 {
                continue;
            }
            match module {
                Sling::Knockback { damage_multiplier, .. } => knockback(release, env, nbt, entry, id, module, *damage_multiplier, charge, &holder, &entities, &active, build_height),
                Sling::Leap { .. } => leap(release, env, nbt, entry, id, module, charge, &holder, &entities, &active, build_height),
                Sling::Teleport { .. } => teleport(release, env, nbt, entry, id, module, charge, &holder, &entities, &active, build_height),
            }
        }
    });
}

/// The holder's predicate facts at the release.
fn holder_state(release: &ReleaseUsing) -> LivingState {
    LivingState {
        on_ground: ON_GROUND.lock().ok().and_then(|map| map.get(&release.player.uuid).copied()).unwrap_or(false),
        elytra_flying: false,
        targeting_block: targeting_block(release),
    }
}

/// The targeting block predicate, read as the holder's block raytrace hitting a block
/// within the bonk's reach.
fn targeting_block(release: &ReleaseUsing) -> bool {
    let world = release.world();
    let mut step = 0.25;
    while step <= RANGE {
        if is_solid(&world, add(release.eye, scale(release.look, step))) {
            return true;
        }
        step += 0.25;
    }
    false
}

/// Whether the block at a point fills its space (any block but air), the suffocation
/// reading the world telling supports.
fn is_solid(world: &World, at: [f64; 3]) -> bool {
    world
        .get_block_state([at[0].floor() as i32, at[1].floor() as i32, at[2].floor() as i32])
        .is_some_and(|state| state.block.to_string() != "minecraft:air")
}

/// Whether a block stands between the eye and `distance` along the look.
fn blocked(world: &World, eye: [f64; 3], look: [f64; 3], distance: f64) -> bool {
    let mut step = 0.25;
    while step < distance {
        if is_solid(world, add(eye, scale(look, step))) {
            return true;
        }
        step += 0.25;
    }
    false
}

/// SlingModule.getDrawtime: `ceil(20 * drawtime multiplier / draw speed)`. The knockback
/// module's own override also multiplies the speed by the attack speed stat for a melee
/// weapon (read there from the melee weapon tag; here, as in the throwing hook, from
/// the tool dealing melee damage at all).
fn drawtime(nbt: &NbtCompound, env: &tool_stack::Environment, entities: &crate::entity_facts::Entities, module: &Sling) -> f32 {
    let mut speed = tool_stack::draw_speed(nbt, env, entities);
    if matches!(module, Sling::Knockback { .. }) && tool_stack::stat(nbt, Stat::AttackDamage) > 0.0 {
        speed *= tool_stack::stat(nbt, Stat::AttackSpeed);
    }
    (20.0 * module.drawtime_multiplier() / speed.max(f32::MIN_POSITIVE)).ceil()
}

/// GeneralInteractionModifierHook.getToolCharge: the bow charge curve over the drawtime.
fn charge(ticks_used: u32, drawtime: f32) -> f32 {
    let charge = ticks_used as f32 / drawtime.max(1.0);
    ((charge * charge + charge * 2.0) / 3.0).min(1.0)
}

/// A gaussian draw (RandomSource.nextGaussian) by Box–Muller over the world's random
/// floats; zero when the world names no randomness.
pub(crate) fn gaussian(world: &World) -> f64 {
    let u1 = f64::from(world.random_float().unwrap_or(0.5)).max(1e-6);
    let u2 = f64::from(world.random_float().unwrap_or(0.5));
    (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos()
}

fn add(a: [f64; 3], b: [f64; 3]) -> [f64; 3] { [a[0] + b[0], a[1] + b[1], a[2] + b[2]] }
fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] { [a[0] - b[0], a[1] - b[1], a[2] - b[2]] }
fn scale(a: [f64; 3], by: f64) -> [f64; 3] { [a[0] * by, a[1] * by, a[2] * by] }
fn dot(a: [f64; 3], b: [f64; 3]) -> f64 { a[0] * b[0] + a[1] * b[1] + a[2] * b[2] }
fn length(a: [f64; 3]) -> f64 { dot(a, a).sqrt() }

/// The failure sound of the modifier the use started with, played when this module is it.
fn fail(release: &ReleaseUsing, active: &Identifier, id: &Identifier, sound: &str, pitch: f32) {
    if active == id {
        release.world().play_sound(sound, release.position, 1.0, pitch);
    }
}

/// The wear a landed sling costs the tool (damageAnimated), as an edit of the hand that
/// held it, and the cooldown (`ReleaseUsing::set_used` for the original's three ticks).
pub(crate) fn spend(release: &ReleaseUsing, amount: i32, cause: &Identifier) {
    let same = |slot: &ItemSlot| slot.item.to_string() == release.stack.item && slot.custom_data == release.stack.custom_data;
    let hand = if release.main_hand.as_ref().is_some_and(|slot| same(slot)) {
        Some((CarriedSlot::Equipment(StackSlot::MainHand), release.main_hand.clone()))
    } else if release.off_hand.as_ref().is_some_and(|slot| same(slot)) {
        Some((CarriedSlot::Equipment(StackSlot::OffHand), release.off_hand.clone()))
    } else {
        None
    };
    if let Some((slot, Some(mut worn))) = hand {
        let damaged = with_held_tool(&release.stack, |env, _tool| tool_slot::damage_slot(&mut worn, env, amount, !release.creative, true, Some(cause), &mut crate::tool_damage_hook::random));
        let broke = matches!(&damaged, Ok(Ok(outcome)) if outcome.broke);
        if damaged.is_ok() {
            // damageAnimated's other half: a release that broke the sling announces it, at the hand that held it.
            if broke {
                if let CarriedSlot::Equipment(stack_slot) = slot {
                    release.player.broadcast_item_break(stack_slot, &crate::brush_hook::stack_of(&worn));
                }
            }
            release.edits.borrow_mut().push(CarriedEdit { slot, stack: worn });
        }
    }
    release.set_used();
}

/// SlingKnockbackModule.sling: bonk the living entity in front of the holder.
#[allow(clippy::too_many_arguments)]
fn knockback(release: &ReleaseUsing, env: &tool_stack::Environment, nbt: &NbtCompound, entry: &Entry, id: &Identifier, module: &Sling, damage_multiplier: f32, charge: f32, holder: &LivingState, entities: &crate::entity_facts::Entities, active: &Identifier, build_height: Option<[i32; 2]>) {
    let world = release.world();
    // ProjectileUtil.getEntityHitResult: the nearest living entity the look ray passes,
    // inside the search box the original builds from the eye along the look, grown by one.
    let self_id = EntityReach::id_of_uuid(release.player.uuid);
    let end = add(release.eye, scale(release.look, RANGE));
    let min = [release.eye[0].min(end[0]) - 1.0, release.eye[1].min(end[1]) - 1.0, release.eye[2].min(end[2]) - 1.0];
    let max = [release.eye[0].max(end[0]) + 1.0, release.eye[1].max(end[1]) + 1.0, release.eye[2].max(end[2]) + 1.0];
    let mut best: Option<(u64, [f64; 3], f64)> = None;
    for entity in EntityReach::in_box(min, max) {
        if entity.living.is_none() || Some(entity.id) == self_id {
            continue;
        }
        let Some(position) = EntityReach::position_of(entity.id) else { continue };
        // The entity's middle, standing in for its box (the reach names positions, not boxes).
        let middle = [position[0], position[1] + 0.9, position[2]];
        let along = dot(sub(middle, release.eye), release.look);
        if !(0.0..=RANGE).contains(&along) {
            continue;
        }
        if length(sub(middle, add(release.eye, scale(release.look, along)))) > 0.8 {
            continue;
        }
        if best.as_ref().is_none_or(|(_, _, other)| along < *other) {
            best = Some((entity.id, position, along));
        }
    }
    let Some((target_id, target_position, along)) = best else {
        fail(release, active, id, "tconstruct:bonk", 1.0);
        return;
    };
    let target_state = LivingState::default();
    if !module.target().matches(&target_state) || blocked(&world, release.eye, release.look, along) {
        fail(release, active, id, "tconstruct:bonk", 1.0);
        return;
    }
    let mut did_bonk = false;
    if damage_multiplier > 0.0 && tool_stack::stat(nbt, Stat::AttackDamage) > 0.0 {
        // The bonk is a real attack (ToolAttackUtil.performAttack) at the charge's
        // cooldown. The port's melee pass has no attack-cooldown input, so the damage is
        // the tool's melee damage scaled by the module, and the IS_BONKING flag the
        // original sets for its own melee hooks is folded into this call: the module's
        // melee damage multiplier applies here directly, and its zero-knockback melee
        // hook has no separate hit to answer.
        let base = 1.0 + tool_stack::stat(nbt, Stat::AttackDamage);
        let amount = tool_stack::melee_damage(nbt, env, base, base, &|_| false, entities) * damage_multiplier;
        if amount > 0.0 && EntityReach::hurt(target_id, amount, "minecraft:player_attack") {
            did_bonk = true;
        }
    }
    let multiplier = charge * module.force_multiplier(entry);
    let force = tool_stack::sling_force(nbt, env, entities, holder, &target_state, id, tool_stack::sling_power(nbt, env, entities, build_height) * multiplier, multiplier);
    if force > 0.0 {
        // The knockback itself: `target.knockback(force, angle)` with the inaccuracy
        // spread, through the entity reach — the game's own knockback applies the
        // target's resistance, and a player target's client is told, as the original's
        // motion packet tells it. The reach applies it at the next rider
        // synchronisation. The original counts the bonk whether or not the resistance
        // eats the push, and so does this.
        let inaccuracy = f64::from(tool_stack::inaccuracy(nbt, env, entities)) * 0.0075;
        let angle = [-release.look[0] + gaussian(&world) * inaccuracy, -release.look[2] + gaussian(&world) * inaccuracy];
        EntityReach::knockback(target_id, f64::from(force), angle[0], angle[1]);
        did_bonk = true;
    }
    if !did_bonk {
        fail(release, active, id, "tconstruct:bonk", 1.0);
        return;
    }
    world.send_particles("tconstruct:bonk", &ParticleData::Plain, [target_position[0], target_position[1] + 0.9, target_position[2]], [0.0, 0.0, 0.0], 0.0, 1);
    world.play_sound("tconstruct:bonk", release.position, 1.0, 0.5);
    spend(release, 1, id);
    // SmeltingModule.afterSlingLaunch: the power becomes the force but is halved when applied, so double it, and
    // divide by the sling's own multiplier.
    crate::smelting_hook::cook_release(release, force * 2.0 / multiplier);
}

/// SlingLeapModule.sling: launch the holder.
#[allow(clippy::too_many_arguments)]
fn leap(release: &ReleaseUsing, env: &tool_stack::Environment, nbt: &NbtCompound, entry: &Entry, id: &Identifier, module: &Sling, charge: f32, holder: &LivingState, entities: &crate::entity_facts::Entities, active: &Identifier, build_height: Option<[i32; 2]>) {
    let world = release.world();
    if !module.target().matches(holder) {
        fail(release, active, id, "tconstruct:slime_sling", 0.5);
        return;
    }
    let computed = module.force_multiplier(entry);
    let multiplier = charge * computed.abs();
    let force = tool_stack::sling_force(nbt, env, entities, holder, holder, id, tool_stack::sling_power(nbt, env, entities, build_height) * multiplier, multiplier);
    if force <= 0.0 {
        fail(release, active, id, "tconstruct:slime_sling", 0.5);
        return;
    }
    // The launch angle, exactly as SlingLeapModule builds it: the upward bias joins the
    // look before it is normalized, the height reduction divides the y after, the
    // inaccuracy spreads every axis, and a negative force multiplier flips the whole
    // angle. The push itself lands through the entity reach — see the header.
    let Sling::Leap { height_reduction, bias_upwards, leave_ground, .. } = module else { return };
    let inaccuracy = f64::from(tool_stack::inaccuracy(nbt, env, entities)) * 0.0075;
    let mut look = release.look;
    if *bias_upwards {
        look[1] += (0.5 - f64::from(force) * 0.1 * f64::from(computed.signum())).max(0.0);
    }
    let len = length(look).max(f64::MIN_POSITIVE);
    let angle = [
        look[0] / len + gaussian(&world) * inaccuracy,
        (look[1] / len + gaussian(&world) * inaccuracy) / f64::from(*height_reduction),
        look[2] / len + gaussian(&world) * inaccuracy,
    ];
    let push = scale(angle, if computed < 0.0 { -f64::from(force) } else { f64::from(force) });
    // The launch: `entity.push(...)` through the entity reach, and the leave-ground hop
    // after it (`entity.move(SELF, (0, 1.3, 0))`) when the module asks for it, so the
    // holder is airborne before the motion meets the ground. Both are queued in this
    // order and land at the host's next rider synchronisation.
    if let Some(holder_id) = EntityReach::id_of_uuid(release.player.uuid) {
        EntityReach::push(holder_id, push);
        if *leave_ground {
            EntityReach::leave_ground(holder_id);
        }
        // The leap's slime bounce handler: the momentum-only handler, so the holder keeps their
        // speed through the air after the launch (the fall handler drives it from there).
        crate::bounce::add_bounce_handler(holder_id as i32);
        // A drill sling's leap ends in a spin attack: the original starts one
        // (`player.startAutoSpinAttack(20)`) when the leaping modifier is in the drill-attacks
        // tag and the tool can perform the drill attack.
        let drill = Identifier::parse("tconstruct:drill_attacks").expect("the drill-attacks tag id parses");
        if env.modifiers.reverse_tags.get(id).is_some_and(|tags| tags.contains(&drill)) && tool_stack::can_perform_action(nbt, env, "drill_attack") {
            EntityReach::spin_attack(holder_id);
        }
    }
    world.play_sound("tconstruct:slime_sling", release.position, 1.0, 1.0);
    spend(release, 1, id);
    crate::smelting_hook::cook_release(release, force * 2.0 / multiplier);
}

/// SlingTeleportModule.sling: move the holder along the look.
#[allow(clippy::too_many_arguments)]
fn teleport(release: &ReleaseUsing, env: &tool_stack::Environment, nbt: &NbtCompound, entry: &Entry, id: &Identifier, module: &Sling, charge: f32, holder: &LivingState, entities: &crate::entity_facts::Entities, active: &Identifier, build_height: Option<[i32; 2]>) {
    let world = release.world();
    if !module.target().matches(holder) {
        fail(release, active, id, "tconstruct:slime_sling.teleport", 0.5);
        return;
    }
    let multiplier = charge * module.force_multiplier(entry);
    let force = tool_stack::sling_force(nbt, env, entities, holder, holder, id, tool_stack::sling_power(nbt, env, entities, build_height) * multiplier, multiplier);
    if force <= 0.0 {
        fail(release, active, id, "tconstruct:slime_sling.teleport", 0.5);
        return;
    }
    // The destination search, as SlingTeleportModule walks it: from the full offset (the
    // look plus the inaccuracy spread, not normalized, one block up) back toward the
    // player in quarter steps, keeping the furthest candidate standing in a block that
    // does not suffocate. Two of the original's checks have no facts here: the world
    // border (no border surface) and clearing the destination on an unbreakable block
    // (the world telling carries no destroy speed). Suffocation reads any non-air block
    // as filling its space, so the search errs short, never long.
    let inaccuracy = f64::from(tool_stack::inaccuracy(nbt, env, entities)) * 0.0075;
    let mut offset = [
        (release.look[0] + gaussian(&world) * inaccuracy) * f64::from(force),
        (release.look[1] + gaussian(&world) * inaccuracy) * f64::from(force) + 1.0,
        (release.look[2] + gaussian(&world) * inaccuracy) * f64::from(force),
    ];
    let mut furthest: Option<[i32; 3]> = None;
    while offset.iter().any(|axis| axis.abs() > 0.5) {
        let candidate = [
            (release.position[0] + offset[0]).floor() as i32,
            (release.position[1] + offset[1]).floor() as i32,
            (release.position[2] + offset[2]).floor() as i32,
        ];
        if furthest.is_none() && !is_solid(&world, [f64::from(candidate[0]), f64::from(candidate[1]), f64::from(candidate[2])]) {
            furthest = Some(candidate);
        }
        for axis in offset.iter_mut() {
            if axis.abs() > 0.25 {
                *axis -= 0.25 * axis.signum();
            }
        }
    }
    let Some(destination) = furthest else {
        fail(release, active, id, "tconstruct:slime_sling.teleport", 0.5);
        return;
    };
    let Some(player_id) = EntityReach::id_of_uuid(release.player.uuid) else {
        fail(release, active, id, "tconstruct:slime_sling.teleport", 0.5);
        return;
    };
    // The reach answers false for a player on this PlutoLib (the host has no call that
    // moves a player), so the wiring stands ready and the success effects key off the
    // answer, as they key off the original's teleport event not being cancelled.
    if !EntityReach::teleport(player_id, [f64::from(destination[0]) + 0.5, f64::from(destination[1]), f64::from(destination[2]) + 0.5]) {
        fail(release, active, id, "tconstruct:slime_sling.teleport", 0.5);
        return;
    }
    world.send_particles("minecraft:portal", &ParticleData::Plain, release.position, [0.5, 0.5, 0.5], 0.5, 32);
    world.play_sound("tconstruct:slime_sling.teleport", release.position, 1.0, 1.0);
    spend(release, 1, id);
    crate::smelting_hook::cook_release(release, force * 2.0 / multiplier);
}
