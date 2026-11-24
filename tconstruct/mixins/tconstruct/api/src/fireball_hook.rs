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
//! - The flight's end below the world: the original discards past the build height;
//!   the flight telling carries no build height, so only the 2400-tick half runs.
use crate::{
    break_speed_hook::with_held_tool, common, modifier_entries::Entry,
    projectile_modules::Fireball, sling_hook, tool_stack, tool_stats::Stat,
};
use pluto_lib::{
    ActionTarget, EntityAction, EntityTypeRegistry, Identifier, ItemEvents, Nbt, NbtCompound,
    ProjectileEvents, ProjectileFlight, ProjectileImpact, ProjectileSpawn, ReleaseUsing,
    StackData, spawn_projectile,
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
    // holder is not digging, so no dig reports them); the velocity alone reads the holder, so the
    // power above keeps the holder-less entities it always had.
    let holder = pluto_lib::EntityState { y: Some(release.position[1]), ..Default::default() };
    let velocity_entities = crate::entity_facts::Entities { attacker: Some(&holder), target: None };
    let build_height = Some(crate::mining_speed::build_height_for(&release.dimension));
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

/// CustomFireball.tick: the flight ends after 2400 ticks.
fn on_flight(flight: &ProjectileFlight) {
    if flight.kind != FIREBALL {
        return;
    }
    if flight.age > 2400 {
        flight.discard.set(true);
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
