//! FireballModule firing and the CustomFireball entity (TinkerModifiers' `fireball`).
//!
//! The entity is a derived type over the game's own fireball body, registered like the
//! fluid spit is: no content declaration, just the registration and the spawn. A use of
//! a tool carrying the module consumes one carried ammo an option matches (the first
//! such stack in the inventory, whose matching option decides the shot's type) and
//! summons the fireball from the player's eyes with the tool's projectile damage as its
//! power. On a hit it hurts by the original's own formula — the power times the damage
//! multiplier times the motion's length, rounded up, through the tool's projectile hit
//! hooks — and discards; the game's own fireball hit never runs. Its saved data carries
//! the original's keys (`damage_power`, `damage_multiplier`, `damage_type`,
//! `enderference_type`) plus the carried item, whose name names the fireball.
//!
//! The firing rides the release telling, not the press: a fireball tool's use is given
//! the long use time in the composed registration, so a click and a hold both end in
//! one release and one shot, where the original shoots at the press and would shoot
//! again per press. What else has no surface yet, exactly:
//! - The helmet path (the module's armor interact): no server telling carries a helmet
//!   interact, so the `tconstruct:fireball_cooldown` effect gate that path applies has
//!   nothing to gate here.
//! - The cooldown: the original adds the drawtime in ticks; `ReleaseUsing::set_used`
//!   starts the item's own content cooldown instead.
//! - The enderference override: a target the enderference effect protects is hurt by
//!   the pair's melee type instead, which needs the target's effect state; the impact
//!   telling does not carry it, so the ranged type always lands.
//! - The shot's added modifiers: the module's and the type's ammo modifiers join the
//!   shot's modifier list in the original (only when the module names any of its own),
//!   and their projectile hooks (bounce and friends) ride the flight profile, which is
//!   built from a fired stack; a summoned fireball has none, so those hooks do not run
//!   for a fireball yet. The fireball's own damage path is complete without them.
//! - The flight's end below the world: the original discards past the build height
//!   (`CustomFireball.checkBelowWorld`: below min - 64 or above max + 64), read from the
//!   host for the flight's dimension; when the host has not published the level's range,
//!   only the 2400-tick half runs.
use crate::{
    break_speed_hook::with_held_tool, common, modifier_entries::Entry,
    projectile_modules::Fireball, sling_hook, tool_stack, tool_stats::Stat,
};
use pluto_lib::{
    ActionTarget, EntityAction, EntityTypeRegistry, Identifier, ItemEvents, Nbt, NbtCompound,
    ProjectileEvents, ProjectileFlight, ProjectileImpact, ProjectileSpawn, ReleaseUsing,
    StackData, World, spawn_projectile,
};
use std::collections::HashMap;
use std::sync::Mutex;

/// The fireball entity type (TinkerModifiers.fireball).
const FIREBALL: &str = "tconstruct:fireball";

/// A shot in flight: what its hit needs that the entity's saved data does not say.
struct Shot {
    power: f32,
    damage_multiplier: f32,
    damage_type: String,
    /// The tool as fired, for its projectile hit hooks at the hit.
    tool: StackData,
}

static SHOTS: Mutex<HashMap<(String, u32), Shot>> = Mutex::new(HashMap::new());

/// Registers the entity type and the firing, flight and impact callbacks. Called once
/// from the product's hooks, after the throwing hook (whose use registration this
/// composes into).
pub(crate) fn register() {
    if let (Ok(kind), Ok(factory)) = (Identifier::parse(FIREBALL), Identifier::parse("minecraft:fireball")) {
        let _ = EntityTypeRegistry::register_derived(kind, factory, 1.0, 1.0, 4, true);
    }
    ItemEvents::RELEASE_USING.register(on_release);
    ProjectileEvents::FLIGHT.register(on_flight);
    ProjectileEvents::IMPACT.register(on_impact);
}

fn on_release(release: &ReleaseUsing) {
    let _ = with_held_tool(&release.stack, |env, tool| {
        let Some((entry, id, module)) = tool_stack::fireball_module(&tool.custom_data, env) else { return };
        shoot(release, env, &tool.custom_data, &entry, &id, &module);
    });
}

/// FireballModule.shoot: find the ammo, spend it, and launch the fireball.
fn shoot(release: &ReleaseUsing, env: &tool_stack::Environment, nbt: &NbtCompound, entry: &Entry, id: &Identifier, module: &Fireball) {
    let player = &release.player;
    // BowAmmoModifierHook.consumeAmmo: the first carried stack an option matches; the
    // option matching it decides the type (getFireballType). A creative shooter still
    // needs the ammo carried — the consume finds it the same way — it just is not spent.
    let snapshot = common::item_snapshot();
    let in_tag = |item: &Identifier, tag: &Identifier| {
        snapshot.as_ref().is_some_and(|lookup| lookup.get_tag(tag).is_some_and(|members| members.contains(item)))
    };
    let mut found = None;
    for slot in 0..36 {
        let Some(stack) = player.stack(slot) else { continue };
        if stack.count == 0 {
            continue;
        }
        let Ok(item) = Identifier::parse(&stack.item) else { continue };
        if let Some(option) = module.option_for(&item, &in_tag) {
            found = Some((slot, stack, option.clone()));
            break;
        }
    }
    let Some((slot, ammo, option)) = found else { return };
    if !release.creative {
        if ammo.count > 1 {
            let mut shrunk = ammo.clone();
            shrunk.count -= 1;
            player.update_stack(slot, &shrunk);
        } else {
            player.take_item(&ammo.item, 1);
        }
    }
    let world = release.world();
    let entities = crate::entity_facts::Entities::default();
    let power = tool_stack::conditional_stat(nbt, env, Stat::ProjectileDamage, &|_| false, &entities);
    // The holder's height and the level's build range, for the dwarven velocity half (a shot's
    // holder is not digging, so no dig reports them): the host's own range for the dimension,
    // the vanilla table when it has not answered.
    let holder = pluto_lib::EntityState { y: Some(release.position[1]), ..Default::default() };
    let velocity_entities = crate::entity_facts::Entities { attacker: Some(&holder), target: None };
    let build_height = Some(crate::mining_speed::level_build_height(&release.dimension));
    let velocity = f64::from(tool_stack::conditional_stat_at(nbt, env, Stat::Velocity, &|_| false, &velocity_entities, build_height));
    let inaccuracy = f64::from(tool_stack::inaccuracy(nbt, env, &entities)) / 16.0;
    // The look scaled by two, jittered on x and z only, the whole scaled by the velocity.
    let motion = [
        (release.look[0] * 2.0 + sling_hook::gaussian(&world) * inaccuracy) * velocity,
        release.look[1] * 2.0 * velocity,
        (release.look[2] * 2.0 + sling_hook::gaussian(&world) * inaccuracy) * velocity,
    ];
    // entity.getY(0.5) + 0.5, with the player's height: feet + 0.9 + 0.5.
    let position = [release.position[0], release.position[1] + 1.4, release.position[2]];
    let (ranged, melee) = option.damage_type.clone().unwrap_or_else(|| module.damage_type.clone());
    let damage_multiplier = module.damage_multiplier * option.damage_multiplier;
    let mut data = NbtCompound::new();
    data.insert("damage_power".into(), Nbt::Float(power));
    data.insert("damage_multiplier".into(), Nbt::Float(damage_multiplier));
    data.insert("damage_type".into(), Nbt::String(ranged.to_string()));
    data.insert("enderference_type".into(), Nbt::String(melee.to_string()));
    // The carried item rides as the entity's item, as setItem leaves it.
    let mut item = NbtCompound::new();
    item.insert("id".into(), Nbt::String(ammo.item.clone()));
    item.insert("count".into(), Nbt::Int(1));
    data.insert("Item".into(), Nbt::Compound(item));
    // The tool's projectile launch hooks record their multipliers in the shot's data,
    // as they do for a shot the game fires.
    tool_stack::power_shoot(nbt, env, false, &mut data);
    let Ok(kind) = Identifier::parse(FIREBALL) else { return };
    let spawn = ProjectileSpawn::new(kind, position, motion).owner(player.uuid).data(data);
    let Some(projectile) = spawn_projectile(&world, &spawn) else { return };
    if let Ok(mut shots) = SHOTS.lock() {
        shots.insert(
            (release.dimension.clone(), projectile),
            Shot { power, damage_multiplier, damage_type: ranged.to_string(), tool: release.stack.clone() },
        );
    }
    let pitch = 1.0 + (world.random_float().unwrap_or(0.5) - world.random_float().unwrap_or(0.5)) * 0.2;
    world.play_sound(&module.sound.to_string(), release.position, 2.0, pitch);
    if release.creative {
        release.set_used();
    } else {
        let usage = module.usage(entry);
        if usage > 0 {
            sling_hook::spend(release, usage, id);
        } else {
            release.set_used();
        }
    }
}

/// CustomFireball.tick: the flight ends after 2400 ticks. CustomFireball.checkBelowWorld
/// discards it past the build height too: below the level's min less 64 or above its max
/// plus 64, read from the host for the flight's dimension (a level publishes its range on
/// every tick, so the flight telling — which runs in no world callback — reads it all the
/// same). When the host has not published the range, only the 2400-tick half runs.
fn on_flight(flight: &ProjectileFlight) {
    if flight.kind != FIREBALL {
        return;
    }
    if flight.age > 2400 {
        flight.discard.set(true);
        return;
    }
    if let Some([min_y, max_y]) = World::new(&flight.dimension).build_height() {
        let y = flight.position[1];
        if y < f64::from(min_y) - 64.0 || y > f64::from(max_y) + 64.0 {
            flight.discard.set(true);
        }
    }
}

/// CustomFireball.onHit: an entity hit hurts and every hit discards, server side.
fn on_impact(impact: &ProjectileImpact) -> bool {
    if impact.kind != FIREBALL {
        return true;
    }
    let shot = SHOTS.lock().ok().and_then(|mut shots| shots.remove(&(impact.dimension.clone(), impact.projectile)));
    if impact.entity.is_some() {
        if let Some(shot) = shot {
            // getDamage: the power times the damage multiplier times the motion's
            // length, rounded up, through the tool's projectile hit hooks.
            let speed = (impact.motion[0].powi(2) + impact.motion[1].powi(2) + impact.motion[2].powi(2)).sqrt();
            let base = (f64::from(shot.power) * f64::from(shot.damage_multiplier) * speed).ceil() as f32;
            let amount = with_held_tool(&shot.tool, |env, tool| {
                let mut data = impact.data.clone();
                tool_stack::power_hit(
                    &tool.custom_data,
                    env,
                    &mut data,
                    &crate::conditional_stat::PowerContext { holder_in_tag: &|_| false, target_in_tag: &|_| false, entities: crate::entity_facts::Entities::default() },
                    base,
                )
            })
            .unwrap_or(base);
            if amount > 0.0 {
                impact.actions.borrow_mut().push(EntityAction::Hurt { target: ActionTarget::Victim, amount, damage_type: shot.damage_type });
            }
        }
    }
    impact.discard.set(true);
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use pluto_lib::{MemoryWorldOps, with_world};

    fn flight(dimension: &str, age: u32, y: f64) -> ProjectileFlight {
        ProjectileFlight {
            dimension: dimension.to_owned(),
            projectile: 1,
            kind: FIREBALL.to_owned(),
            owner: -1,
            owner_position: None,
            age,
            position: [0.0, y, 0.0],
            motion: [0.0; 3],
            in_ground: false,
            no_gravity: false,
            arrow: None,
            data: NbtCompound::new(),
            motion_after: std::cell::RefCell::new(None),
            discard: std::cell::Cell::new(false),
            no_gravity_after: std::cell::Cell::new(None),
            release_from_ground: std::cell::Cell::new(false),
            actions: std::cell::RefCell::new(Vec::new()),
        }
    }

    #[test]
    fn a_fireball_discards_past_the_build_height_and_not_inside_it() {
        // The overworld's range, and a custom dimension's own range, as the host publishes them.
        let mut ops = MemoryWorldOps::default();
        ops.build_heights.insert("minecraft:overworld".into(), [-64, 320]);
        ops.build_heights.insert("example:custom".into(), [-32, 128]);
        with_world(&mut ops, || {
            // Inside the overworld's range and its 64-block margin it flies on.
            let flying = flight("minecraft:overworld", 10, 383.0);
            on_flight(&flying);
            assert!(!flying.discard.get(), "384 is the top of the margin, 383 still flies");
            // Past the top margin it ends: max 320 + 64 = 384.
            let high = flight("minecraft:overworld", 10, 384.5);
            on_flight(&high);
            assert!(high.discard.get());
            // Below the bottom margin it ends too: min -64 - 64 = -128.
            let low = flight("minecraft:overworld", 10, -128.5);
            on_flight(&low);
            assert!(low.discard.get());
            // A custom dimension reads its own range: its top margin is 128 + 64 = 192.
            let custom_inside = flight("example:custom", 10, 191.0);
            on_flight(&custom_inside);
            assert!(!custom_inside.discard.get());
            let custom_past = flight("example:custom", 10, 192.5);
            on_flight(&custom_past);
            assert!(custom_past.discard.get());
            // The 2400-tick half still ends a flight inside the range.
            let old = flight("minecraft:overworld", 2401, 64.0);
            on_flight(&old);
            assert!(old.discard.get());
        });
        // With no range published for the dimension, only the 2400-tick half runs.
        let unknown = flight("example:unpublished", 10, 9999.0);
        on_flight(&unknown);
        assert!(!unknown.discard.get(), "no host range: a high flight is not discarded");
        let unknown_old = flight("example:unpublished", 2401, 64.0);
        on_flight(&unknown_old);
        assert!(unknown_old.discard.get());
        // Another kind is not a fireball: the flight is left alone.
        let mut other = flight("minecraft:overworld", 10, 9999.0);
        other.kind = "minecraft:arrow".into();
        on_flight(&other);
        assert!(!other.discard.get());
    }
}
